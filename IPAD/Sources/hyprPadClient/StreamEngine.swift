import CRustCore
import Combine
import Foundation

class StreamEngine: ObservableObject, @unchecked Sendable {
    @Published var isStreaming = false
    @Published var fps: Double = 0
    @Published var bitrateMbps: Double = 0
    @Published var latencyMs: Double = 0
    @Published var decoderDebug: String = "Wachten op decoder..."

    let decoder = H264Decoder()

    private var statsTimer: DispatchSourceTimer?
    private var bitrateRefBytes: UInt64 = 0
    private var bitrateRefDate: Date = Date()

    func startListening() {
        guard !isStreaming else { return }
        isStreaming = true

        // 1. Koppel de decoder veilig aan de globale bridge
        GlobalBridge.shared.setDecoder(decoder)

        // 2. Koppel de debug callback aan onze @Published variabele
        decoder.onStatusUpdate = { [weak self] status in
            DispatchQueue.main.async {
                self?.decoderDebug = status
            }
        }

        // 3. Definieer de C-FFI Callbacks (DIRECT naar de bridge!)
        let onNalu:
            @convention(c) (UnsafePointer<UInt8>?, UInt32, UInt8, UnsafeMutableRawPointer?) -> Void =
                { ptr, len, nalType, _ in
                    GlobalBridge.shared.handleNalu(data: ptr, len: len, nalType: nalType)
                }

        let onLog: @convention(c) (UInt8, UnsafePointer<CChar>?, UnsafeMutableRawPointer?) -> Void =
            { level, msgPtr, _ in
                guard let msgPtr = msgPtr else { return }
                print("🦀 Rust [\(level)]: \(String(cString: msgPtr))")
            }

        // 4. Verpak ze in de C struct
        let callbacks = HyprpadCallbacks(on_nalu: onNalu)

        // 5. Start de Rust Engine op poort 5000!
        let success = hyprpad_start(5000, callbacks, nil)

        if success {
            print("✅ Rust Core succesvol gestart op poort 5000!")
            startStatsMonitoring()
        } else {
            print("❌ Rust Core kon niet starten.")
            isStreaming = false
        }
    }

    func stopListening() {
        isStreaming = false
        hyprpad_stop()
        stopStatsMonitoring()

        decoder.reset()
        GlobalBridge.shared.setDecoder(nil)

        fps = 0
        bitrateMbps = 0
        latencyMs = 0
        decoderDebug = "Wachten op decoder..."
    }

    // MARK: - Stats Polling
    private func startStatsMonitoring() {
        bitrateRefBytes = 0
        bitrateRefDate = Date()

        let timer = DispatchSource.makeTimerSource(queue: .main)
        timer.schedule(deadline: .now() + 1, repeating: 1.0)
        timer.setEventHandler { [weak self] in
            guard let self = self else { return }

            let stats = hyprpad_stats()
            self.fps = Double(stats.fps)

            let bytesTotal = stats.bytes_total
            let now = Date()
            let elapsed = now.timeIntervalSince(self.bitrateRefDate)
            if elapsed > 0.4 {
                let delta = bytesTotal - self.bitrateRefBytes
                self.bitrateMbps = (Double(delta) * 8) / (elapsed * 1_000_000)
                self.bitrateRefDate = now
                self.bitrateRefBytes = bytesTotal
            }
        }
        timer.resume()
        statsTimer = timer
    }

    private func stopStatsMonitoring() {
        statsTimer?.cancel()
        statsTimer = nil
    }
}
