// //
// //  H264Decoder.swift
// //
// //  Reassembles slice-threaded H.264 (Annex-B) NAL units into complete access
// //  units before handing them to VideoToolbox.
// //
// //  Why: with libx264's `superfast` preset running on N encoder threads, each
// //  encoded frame is split into N independent slice NAL units (slice-based
// //  threading), so N NALUs cross the wire per frame instead of 1. Feeding each
// //  slice to VTDecompressionSession as its own sample makes VideoToolbox treat
// //  every slice as a whole frame — hence 960fps (16 slices x 60fps zendfrequentie)
// //  instead of 60 real frames.
// //
// //  Fix: buffer incoming slice NALUs. An Access Unit Delimiter (AUD, NAL type 9)
// //  marks the start of the NEXT access unit, so whatever is buffered when an AUD
// //  arrives is the previous, now-complete frame — flush it as a single
// //  CMSampleBuffer (all slices concatenated, AVCC length-prefixed), then start
// //  buffering the new one.
// //
// //  Usage:
// //    let decoder = H264Decoder()
// //    decoder.onDecodedFrame = { pixelBuffer, pts in /* render */ }
// //    decoder.decode(nalUnit: payload)        // one Annex-B-stripped NALU per call
// //    decoder.decode(annexBData: datagram)    // or a raw chunk with start codes
// //

// import CoreMedia
// import CoreVideo
// import Foundation
// import VideoToolbox

// final class H264Decoder {

//     // MARK: - Public API

//     /// Fired with every fully decoded frame, on `callbackQueue`.
//     var onDecodedFrame: ((CVPixelBuffer, CMTime) -> Void)?

//     init(callbackQueue: DispatchQueue = .main) {
//         self.callbackQueue = callbackQueue
//     }

//     deinit {
//         if let session {
//             VTDecompressionSessionInvalidate(session)
//         }
//     }

//     /// Feed one NAL unit, WITHOUT its Annex-B start code (00 00 01 / 00 00 00 01).
//     func decode(nalUnit: Data) {
//         decodeQueue.async { [weak self] in
//             self?.handle(nalUnit: nalUnit)
//         }
//     }

//     /// Feed a raw chunk that may contain one or more start-code-delimited NAL units
//     /// (e.g. straight off a UDP socket, if several NALUs are batched per datagram).
//     func decode(annexBData data: Data) {
//         decodeQueue.async { [weak self] in
//             for nalu in H264Decoder.splitAnnexB(data) {
//                 self?.handle(nalUnit: nalu)
//             }
//         }
//     }

//     /// Drop any partially-buffered access unit. Call after a stream discontinuity
//     /// (reconnect / detected packet-loss gap) so a half-received frame is never
//     /// flushed as if it were complete.
//     func reset() {
//         decodeQueue.async { [weak self] in
//             self?.pendingAccessUnitData.removeAll(keepingCapacity: true)
//             self?.pendingAccessUnitContainsIDR = false
//         }
//     }

//     // MARK: - Queue / state

//     private let decodeQueue = DispatchQueue(label: "h264decoder.decode")
//     private let callbackQueue: DispatchQueue

//     private var session: VTDecompressionSession?
//     private var formatDescription: CMFormatDescription?

//     private var spsData: Data?
//     private var ppsData: Data?

//     /// Slice NALUs (AVCC length-prefixed) for the access unit currently being assembled.
//     private var pendingAccessUnitData = Data()
//     private var pendingAccessUnitContainsIDR = false

//     private var frameIndex: Int64 = 0
//     private let frameDuration = CMTime(value: 1, timescale: 60)  // 60fps zendfrequentie

//     // MARK: - NALU dispatch

//     private func handle(nalUnit: Data) {
//         guard let firstByte = nalUnit.first else { return }
//         let nalType = firstByte & 0x1F

//         switch nalType {
//         case 7:  // SPS
//             spsData = nalUnit
//             tryUpdateFormatDescription()

//         case 8:  // PPS
//             ppsData = nalUnit
//             tryUpdateFormatDescription()

//         case 9:  // AUD — start of the NEXT access unit.
//             // Whatever is buffered right now is the previous, now-complete frame.
//             flushPendingAccessUnit()

//         default:  // slice data (1 = non-IDR, 5 = IDR), SEI (6), etc.
//             appendToPendingAccessUnit(nalUnit, nalType: nalType)
//         }
//     }

//     private func appendToPendingAccessUnit(_ nalUnit: Data, nalType: UInt8) {
//         if nalType == 5 { pendingAccessUnitContainsIDR = true }

//         // AVCC framing: VideoToolbox expects each NALU in the sample prefixed
//         // with its length as a 4-byte big-endian integer (no start codes).
//         var length = UInt32(nalUnit.count).bigEndian
//         withUnsafeBytes(of: &length) { pendingAccessUnitData.append(contentsOf: $0) }
//         pendingAccessUnitData.append(nalUnit)
//     }

//     private func flushPendingAccessUnit() {
//         defer {
//             pendingAccessUnitData.removeAll(keepingCapacity: true)
//             pendingAccessUnitContainsIDR = false
//         }

//         guard !pendingAccessUnitData.isEmpty else { return }
//         guard let formatDescription else { return }  // no SPS/PPS seen yet

//         submit(
//             accessUnit: pendingAccessUnitData,
//             isIDR: pendingAccessUnitContainsIDR,
//             formatDescription: formatDescription)
//     }

//     // MARK: - Format description (SPS/PPS)

//     private func tryUpdateFormatDescription() {
//         guard let spsData, let ppsData else { return }

//         spsData.withUnsafeBytes { (spsRaw: UnsafeRawBufferPointer) in
//             ppsData.withUnsafeBytes { (ppsRaw: UnsafeRawBufferPointer) in
//                 guard let spsPtr = spsRaw.bindMemory(to: UInt8.self).baseAddress,
//                     let ppsPtr = ppsRaw.bindMemory(to: UInt8.self).baseAddress
//                 else {
//                     return
//                 }

//                 let pointers: [UnsafePointer<UInt8>] = [spsPtr, ppsPtr]
//                 let sizes: [Int] = [spsData.count, ppsData.count]

//                 var newFormatDescription: CMFormatDescription?
//                 let status = CMVideoFormatDescriptionCreateFromH264ParameterSets(
//                     allocator: kCFAllocatorDefault,
//                     parameterSetCount: 2,
//                     parameterSetPointers: pointers,
//                     parameterSetSizes: sizes,
//                     nalUnitHeaderLength: 4,
//                     formatDescriptionOut: &newFormatDescription
//                 )

//                 if status == noErr, let newFormatDescription {
//                     self.formatDescription = newFormatDescription
//                 } else if status != noErr {
//                     print("H264Decoder: format description creation failed (\(status))")
//                 }
//             }
//         }
//     }

//     // MARK: - VTDecompressionSession

//     private func decompressionSession(for formatDescription: CMFormatDescription)
//         -> VTDecompressionSession?
//     {
//         if let session,
//             VTDecompressionSessionCanAcceptFormatDescription(
//                 session, formatDescription: formatDescription)
//         {
//             return session
//         }

//         if let session {
//             VTDecompressionSessionInvalidate(session)
//         }
//         session = nil

//         let imageBufferAttributes: [CFString: Any] = [
//             kCVPixelBufferPixelFormatTypeKey: kCVPixelFormatType_420YpCbCr8BiPlanarFullRange,
//             kCVPixelBufferMetalCompatibilityKey: true,
//         ]

//         var callback = VTDecompressionOutputCallbackRecord(
//             decompressionOutputCallback: h264DecoderOutputCallback,
//             decompressionOutputRefCon: Unmanaged.passUnretained(self).toOpaque()
//         )

//         var newSession: VTDecompressionSession?
//         let status = VTDecompressionSessionCreate(
//             allocator: kCFAllocatorDefault,
//             formatDescription: formatDescription,
//             decoderSpecification: nil,
//             imageBufferAttributes: imageBufferAttributes as CFDictionary,
//             outputCallback: &callback,
//             decompressionSessionOut: &newSession
//         )

//         guard status == noErr, let newSession else {
//             print("H264Decoder: VTDecompressionSessionCreate failed (\(status))")
//             return nil
//         }

//         VTSessionSetProperty(
//             newSession, key: kVTDecompressionPropertyKey_RealTime, value: kCFBooleanTrue)

//         session = newSession
//         return newSession
//     }

//     private func submit(accessUnit data: Data, isIDR: Bool, formatDescription: CMFormatDescription)
//     {
//         guard let session = decompressionSession(for: formatDescription) else { return }

//         var blockBuffer: CMBlockBuffer?
//         let blockStatus: OSStatus = data.withUnsafeBytes {
//             (raw: UnsafeRawBufferPointer) -> OSStatus in
//             guard let source = raw.baseAddress else {
//                 return kCMBlockBufferStructureAllocationFailedErr
//             }
//             guard let copy = malloc(data.count) else {
//                 return kCMBlockBufferStructureAllocationFailedErr
//             }
//             memcpy(copy, source, data.count)

//             let status = CMBlockBufferCreateWithMemoryBlock(
//                 allocator: kCFAllocatorDefault,
//                 memoryBlock: copy,
//                 blockLength: data.count,
//                 blockAllocator: kCFAllocatorDefault,  // takes ownership of `copy` on success
//                 customBlockSource: nil,
//                 offsetToData: 0,
//                 dataLength: data.count,
//                 flags: 0,
//                 blockBufferOut: &blockBuffer
//             )
//             if status != kCMBlockBufferNoErr { free(copy) }
//             return status
//         }

//         guard blockStatus == kCMBlockBufferNoErr, let blockBuffer else {
//             print("H264Decoder: CMBlockBufferCreateWithMemoryBlock failed (\(blockStatus))")
//             return
//         }

//         var timingInfo = CMSampleTimingInfo(
//             duration: frameDuration,
//             presentationTimeStamp: CMTime(value: frameIndex, timescale: 60),
//             decodeTimeStamp: .invalid
//         )
//         frameIndex += 1

//         var sampleSize = data.count
//         var sampleBuffer: CMSampleBuffer?
//         let sbStatus = CMSampleBufferCreate(
//             allocator: kCFAllocatorDefault,
//             dataBuffer: blockBuffer,
//             dataReady: true,
//             makeDataReadyCallback: nil,
//             refcon: nil,
//             formatDescription: formatDescription,
//             sampleCount: 1,
//             sampleTimingEntryCount: 1,
//             sampleTimingArray: &timingInfo,
//             sampleSizeEntryCount: 1,
//             sampleSizeArray: &sampleSize,
//             sampleBufferOut: &sampleBuffer
//         )

//         guard sbStatus == noErr, let sampleBuffer else {
//             print("H264Decoder: CMSampleBufferCreate failed (\(sbStatus))")
//             return
//         }

//         if !isIDR,
//             let attachments = CMSampleBufferGetSampleAttachmentsArray(
//                 sampleBuffer, createIfNecessary: true)
//         {
//             let dict = unsafeBitCast(
//                 CFArrayGetValueAtIndex(attachments, 0), to: CFMutableDictionary.self)
//             CFDictionarySetValue(
//                 dict,
//                 Unmanaged.passUnretained(kCMSampleAttachmentKey_NotSync).toOpaque(),
//                 Unmanaged.passUnretained(kCFBooleanTrue).toOpaque())
//         }

//         var infoFlags = VTDecodeInfoFlags()
//         let decodeStatus = VTDecompressionSessionDecodeFrame(
//             session,
//             sampleBuffer: sampleBuffer,
//             flags: [],  // synchronous on decodeQueue -> deterministic decode order, lowest complexity
//             frameRefcon: nil,
//             infoFlagsOut: &infoFlags
//         )

//         if decodeStatus != noErr {
//             print("H264Decoder: VTDecompressionSessionDecodeFrame failed (\(decodeStatus))")
//         }
//     }

//     // MARK: - Annex-B splitting

//     private static func splitAnnexB(_ data: Data) -> [Data] {
//         let bytes = [UInt8](data)
//         guard bytes.count > 3 else { return [] }

//         var startCodes: [(start: Int, length: Int)] = []
//         var i = 0
//         while i + 2 < bytes.count {
//             if bytes[i] == 0, bytes[i + 1] == 0, bytes[i + 2] == 1 {
//                 if i > 0, bytes[i - 1] == 0 {
//                     startCodes.append((start: i - 1, length: 4))
//                 } else {
//                     startCodes.append((start: i, length: 3))
//                 }
//                 i += 3
//             } else {
//                 i += 1
//             }
//         }
//         guard !startCodes.isEmpty else { return [] }

//         var nalUnits: [Data] = []
//         nalUnits.reserveCapacity(startCodes.count)
//         for (index, code) in startCodes.enumerated() {
//             let naluStart = code.start + code.length
//             let naluEnd = index + 1 < startCodes.count ? startCodes[index + 1].start : bytes.count
//             guard naluStart < naluEnd else { continue }
//             nalUnits.append(Data(bytes[naluStart..<naluEnd]))
//         }
//         return nalUnits
//     }

//     // MARK: - Decoded frame delivery

//     fileprivate func deliver(pixelBuffer: CVPixelBuffer, presentationTimeStamp: CMTime) {
//         callbackQueue.async { [weak self] in
//             self?.onDecodedFrame?(pixelBuffer, presentationTimeStamp)
//         }
//     }
// }

// // MARK: - VTDecompressionSession output callback (must be a context-free C function)

// private func h264DecoderOutputCallback(
//     decompressionOutputRefCon: UnsafeMutableRawPointer?,
//     sourceFrameRefCon: UnsafeMutableRawPointer?,
//     status: OSStatus,
//     infoFlags: VTDecodeInfoFlags,
//     imageBuffer: CVImageBuffer?,
//     presentationTimeStamp: CMTime,
//     presentationDuration: CMTime
// ) {
//     guard status == noErr, let imageBuffer, let refCon = decompressionOutputRefCon else { return }
//     let decoder = Unmanaged<H264Decoder>.fromOpaque(refCon).takeUnretainedValue()
//     decoder.deliver(pixelBuffer: imageBuffer, presentationTimeStamp: presentationTimeStamp)
// }

import AVFoundation
import CoreMedia
import CoreVideo
import Foundation
import VideoToolbox

final class H264Decoder {

    // MARK: - Public API

    /// Fired with every fully decoded frame, on `callbackQueue`.
    var onDecodedFrame: ((CVPixelBuffer, CMTime) -> Void)?

    /// Debug status updates for UI / logging.
    var onStatusUpdate: ((String) -> Void)?

    /// Sample buffer display layer for rendering direct VT frames in SwiftUI / UIKit.
    let displayLayer = AVSampleBufferDisplayLayer()

    init(callbackQueue: DispatchQueue = .main) {
        self.callbackQueue = callbackQueue
    }

    deinit {
        if let session {
            VTDecompressionSessionInvalidate(session)
        }
    }

    /// Entry point for C-bridges or network handlers passing raw data and NAL unit types.
    func dispatch(_ data: Data, nalType: UInt8? = nil) {
        // Decode raw data (handles Annex-B chunks or single NALUs)
        if data.starts(with: [0, 0, 0, 1]) || data.starts(with: [0, 0, 1]) {
            decode(annexBData: data)
        } else {
            decode(nalUnit: data)
        }
    }

    /// Feed one NAL unit, WITHOUT its Annex-B start code (00 00 01 / 00 00 00 01).
    func decode(nalUnit: Data) {
        decodeQueue.async { [weak self] in
            self?.handle(nalUnit: nalUnit)
        }
    }

    /// Feed a raw chunk that may contain one or more start-code-delimited NAL units
    /// (e.g. straight off a UDP socket, if several NALUs are batched per datagram).
    func decode(annexBData data: Data) {
        decodeQueue.async { [weak self] in
            for nalu in H264Decoder.splitAnnexB(data) {
                self?.handle(nalUnit: nalu)
            }
        }
    }

    /// Drop any partially-buffered access unit. Call after a stream discontinuity
    /// (reconnect / detected packet-loss gap) so a half-received frame is never
    /// flushed as if it were complete.
    func reset() {
        decodeQueue.async { [weak self] in
            self?.pendingAccessUnitData.removeAll(keepingCapacity: true)
            self?.pendingAccessUnitContainsIDR = false
        }
    }

    // MARK: - Queue / state

    private let decodeQueue = DispatchQueue(label: "h264decoder.decode")
    private let callbackQueue: DispatchQueue

    private var session: VTDecompressionSession?
    private var formatDescription: CMFormatDescription?

    private var spsData: Data?
    private var ppsData: Data?

    /// Slice NALUs (AVCC length-prefixed) for the access unit currently being assembled.
    private var pendingAccessUnitData = Data()
    private var pendingAccessUnitContainsIDR = false

    private var frameIndex: Int64 = 0
    private let frameDuration = CMTime(value: 1, timescale: 60)  // 60fps zendfrequentie

    // MARK: - NALU dispatch

    private func handle(nalUnit: Data) {
        guard let firstByte = nalUnit.first else { return }
        let nalType = firstByte & 0x1F

        switch nalType {
        case 7:  // SPS
            spsData = nalUnit
            tryUpdateFormatDescription()
        // notifyStatus("SPS received")

        case 8:  // PPS
            ppsData = nalUnit
            tryUpdateFormatDescription()
        // notifyStatus("PPS received")

        case 9:  // AUD — start of the NEXT access unit.
            // Whatever is buffered right now is the previous, now-complete frame.
            flushPendingAccessUnit()

        default:  // slice data (1 = non-IDR, 5 = IDR), SEI (6), etc.
            appendToPendingAccessUnit(nalUnit, nalType: nalType)
        }
    }

    private func appendToPendingAccessUnit(_ nalUnit: Data, nalType: UInt8) {
        if nalType == 5 { pendingAccessUnitContainsIDR = true }

        // AVCC framing: VideoToolbox expects each NALU in the sample prefixed
        // with its length as a 4-byte big-endian integer (no start codes).
        var length = UInt32(nalUnit.count).bigEndian
        withUnsafeBytes(of: &length) { pendingAccessUnitData.append(contentsOf: $0) }
        pendingAccessUnitData.append(nalUnit)
    }

    private func flushPendingAccessUnit() {
        defer {
            pendingAccessUnitData.removeAll(keepingCapacity: true)
            pendingAccessUnitContainsIDR = false
        }

        guard !pendingAccessUnitData.isEmpty else { return }
        guard let formatDescription else { return }  // no SPS/PPS seen yet

        submit(
            accessUnit: pendingAccessUnitData,
            isIDR: pendingAccessUnitContainsIDR,
            formatDescription: formatDescription)
    }

    // MARK: - Format description (SPS/PPS)

    private func tryUpdateFormatDescription() {
        guard let spsData, let ppsData else { return }

        spsData.withUnsafeBytes { (spsRaw: UnsafeRawBufferPointer) in
            ppsData.withUnsafeBytes { (ppsRaw: UnsafeRawBufferPointer) in
                guard let spsPtr = spsRaw.bindMemory(to: UInt8.self).baseAddress,
                    let ppsPtr = ppsRaw.bindMemory(to: UInt8.self).baseAddress
                else {
                    return
                }

                let pointers: [UnsafePointer<UInt8>] = [spsPtr, ppsPtr]
                let sizes: [Int] = [spsData.count, ppsData.count]

                var newFormatDescription: CMFormatDescription?
                let status = CMVideoFormatDescriptionCreateFromH264ParameterSets(
                    allocator: kCFAllocatorDefault,
                    parameterSetCount: 2,
                    parameterSetPointers: pointers,
                    parameterSetSizes: sizes,
                    nalUnitHeaderLength: 4,
                    formatDescriptionOut: &newFormatDescription
                )

                if status == noErr, let newFormatDescription {
                    self.formatDescription = newFormatDescription
                    // notifyStatus("Format description created successfully")
                } else if status != noErr {
                    // notifyStatus("Format description error: \(status)")
                }
            }
        }
    }

    // MARK: - VTDecompressionSession

    private func decompressionSession(for formatDescription: CMFormatDescription)
        -> VTDecompressionSession?
    {
        if let session,
            VTDecompressionSessionCanAcceptFormatDescription(
                session, formatDescription: formatDescription)
        {
            return session
        }

        if let session {
            VTDecompressionSessionInvalidate(session)
        }
        session = nil

        let imageBufferAttributes: [CFString: Any] = [
            kCVPixelBufferPixelFormatTypeKey: kCVPixelFormatType_420YpCbCr8BiPlanarFullRange,
            kCVPixelBufferMetalCompatibilityKey: true,
        ]

        var callback = VTDecompressionOutputCallbackRecord(
            decompressionOutputCallback: h264DecoderOutputCallback,
            decompressionOutputRefCon: Unmanaged.passUnretained(self).toOpaque()
        )

        var newSession: VTDecompressionSession?
        let status = VTDecompressionSessionCreate(
            allocator: kCFAllocatorDefault,
            formatDescription: formatDescription,
            decoderSpecification: nil,
            imageBufferAttributes: imageBufferAttributes as CFDictionary,
            outputCallback: &callback,
            decompressionSessionOut: &newSession
        )

        guard status == noErr, let newSession else {
            // notifyStatus("VTDecompressionSessionCreate failed: \(status)")
            return nil
        }

        VTSessionSetProperty(
            newSession, key: kVTDecompressionPropertyKey_RealTime, value: kCFBooleanTrue)

        session = newSession
        return newSession
    }

    private func submit(accessUnit data: Data, isIDR: Bool, formatDescription: CMFormatDescription)
    {
        guard let session = decompressionSession(for: formatDescription) else { return }

        var blockBuffer: CMBlockBuffer?
        let blockStatus: OSStatus = data.withUnsafeBytes {
            (raw: UnsafeRawBufferPointer) -> OSStatus in
            guard let source = raw.baseAddress else {
                return kCMBlockBufferStructureAllocationFailedErr
            }
            guard let copy = malloc(data.count) else {
                return kCMBlockBufferStructureAllocationFailedErr
            }
            memcpy(copy, source, data.count)

            let status = CMBlockBufferCreateWithMemoryBlock(
                allocator: kCFAllocatorDefault,
                memoryBlock: copy,
                blockLength: data.count,
                blockAllocator: kCFAllocatorDefault,  // takes ownership of `copy` on success
                customBlockSource: nil,
                offsetToData: 0,
                dataLength: data.count,
                flags: 0,
                blockBufferOut: &blockBuffer
            )
            if status != kCMBlockBufferNoErr { free(copy) }
            return status
        }

        guard blockStatus == kCMBlockBufferNoErr, let blockBuffer else {
            // notifyStatus("CMBlockBuffer error: \(blockStatus)")
            return
        }

        var timingInfo = CMSampleTimingInfo(
            duration: frameDuration,
            presentationTimeStamp: CMTime(value: frameIndex, timescale: 60),
            decodeTimeStamp: .invalid
        )
        frameIndex += 1

        var sampleSize = data.count
        var sampleBuffer: CMSampleBuffer?
        let sbStatus = CMSampleBufferCreate(
            allocator: kCFAllocatorDefault,
            dataBuffer: blockBuffer,
            dataReady: true,
            makeDataReadyCallback: nil,
            refcon: nil,
            formatDescription: formatDescription,
            sampleCount: 1,
            sampleTimingEntryCount: 1,
            sampleTimingArray: &timingInfo,
            sampleSizeEntryCount: 1,
            sampleSizeArray: &sampleSize,
            sampleBufferOut: &sampleBuffer
        )

        guard sbStatus == noErr, let sampleBuffer else {
            // notifyStatus("CMSampleBuffer error: \(sbStatus)")
            return
        }

        if !isIDR,
            let attachments = CMSampleBufferGetSampleAttachmentsArray(
                sampleBuffer, createIfNecessary: true)
        {
            let dict = unsafeBitCast(
                CFArrayGetValueAtIndex(attachments, 0), to: CFMutableDictionary.self)
            CFDictionarySetValue(
                dict,
                Unmanaged.passUnretained(kCMSampleAttachmentKey_NotSync).toOpaque(),
                Unmanaged.passUnretained(kCFBooleanTrue).toOpaque())
        }

        // Enqueue to AVSampleBufferDisplayLayer if displayLayer is being used directly
        if displayLayer.status == .failed {
            displayLayer.flush()
        }
        displayLayer.enqueue(sampleBuffer)

        var infoFlags = VTDecodeInfoFlags()
        let decodeStatus = VTDecompressionSessionDecodeFrame(
            session,
            sampleBuffer: sampleBuffer,
            flags: [],  // synchronous on decodeQueue -> deterministic decode order, lowest complexity
            frameRefcon: nil,
            infoFlagsOut: &infoFlags
        )

        if decodeStatus != noErr {
            // notifyStatus("Decode error: \(decodeStatus)")
        }
    }

    // MARK: - Annex-B splitting

    private static func splitAnnexB(_ data: Data) -> [Data] {
        let bytes = [UInt8](data)
        guard bytes.count > 3 else { return [] }

        var startCodes: [(start: Int, length: Int)] = []
        var i = 0
        while i + 2 < bytes.count {
            if bytes[i] == 0, bytes[i + 1] == 0, bytes[i + 2] == 1 {
                if i > 0, bytes[i - 1] == 0 {
                    startCodes.append((start: i - 1, length: 4))
                } else {
                    startCodes.append((start: i, length: 3))
                }
                i += 3
            } else {
                i += 1
            }
        }
        guard !startCodes.isEmpty else { return [] }

        var nalUnits: [Data] = []
        nalUnits.reserveCapacity(startCodes.count)
        for (index, code) in startCodes.enumerated() {
            let naluStart = code.start + code.length
            let naluEnd = index + 1 < startCodes.count ? startCodes[index + 1].start : bytes.count
            guard naluStart < naluEnd else { continue }
            nalUnits.append(Data(bytes[naluStart..<naluEnd]))
        }
        return nalUnits
    }

    // MARK: - Helpers & Callbacks

    private func notifyStatus(_ status: String) {
        callbackQueue.async { [weak self] in
            self?.onStatusUpdate?(status)
        }
    }

    fileprivate func deliver(pixelBuffer: CVPixelBuffer, presentationTimeStamp: CMTime) {
        callbackQueue.async { [weak self] in
            self?.onDecodedFrame?(pixelBuffer, presentationTimeStamp)
        }
    }
}

// MARK: - VTDecompressionSession output callback

private func h264DecoderOutputCallback(
    decompressionOutputRefCon: UnsafeMutableRawPointer?,
    sourceFrameRefCon: UnsafeMutableRawPointer?,
    status: OSStatus,
    infoFlags: VTDecodeInfoFlags,
    imageBuffer: CVImageBuffer?,
    presentationTimeStamp: CMTime,
    presentationDuration: CMTime
) {
    guard status == noErr, let imageBuffer, let refCon = decompressionOutputRefCon else { return }
    let decoder = Unmanaged<H264Decoder>.fromOpaque(refCon).takeUnretainedValue()
    decoder.deliver(pixelBuffer: imageBuffer, presentationTimeStamp: presentationTimeStamp)
}
