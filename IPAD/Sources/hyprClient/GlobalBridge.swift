import CRustCore
import Foundation

// Een thread-safe singleton om de globale decoder variabele te beschermen
final class GlobalBridge: @unchecked Sendable {
    static let shared = GlobalBridge()
    private let lock = NSLock()
    private var decoder: H264Decoder?

    private init() {}

    func setDecoder(_ decoder: H264Decoder?) {
        lock.lock()
        self.decoder = decoder
        lock.unlock()
    }

    func handleNalu(data: UnsafePointer<UInt8>?, len: UInt32, nalType: UInt8) {
        guard let data = data else { return }
        lock.lock()
        let decoder = self.decoder
        lock.unlock()

        decoder?.dispatch(Data(bytes: data, count: Int(len)), nalType: nalType)
    }
}

// // De C-callback functie die Rust aanroept voor NAL units
// @_cdecl("rust_on_nalu")
// public func rust_on_nalu(
//     data: UnsafePointer<UInt8>?, len: UInt32, nalType: UInt8, ctx: UnsafeMutableRawPointer?
// ) {
//     GlobalBridge.shared.handleNalu(data: data, len: len, nalType: nalType)
// }

// // De C-callback functie die Rust aanroept voor Logs
// @_cdecl("rust_on_log")
// public func rust_on_log(level: UInt8, msg: UnsafePointer<CChar>?, ctx: UnsafeMutableRawPointer?) {
//     guard let msg = msg else { return }
//     let str = String(cString: msg)
//     print("🦀 Rust [\(level)]: \(str)")
// }
