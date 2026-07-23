import Combine
import Foundation
import Network

class StreamEngine: ObservableObject, @unchecked Sendable {
    @Published var isStreaming = false
    @Published var fps: Double = 0
    @Published var bitrateMbps: Double = 0
    @Published var latencyMs: Double = 0

    // Jouw bestaande decoder
    let decoder = H264Decoder()

    private var listener: NWListener?
    private var statsTimer: DispatchSourceTimer?

    // Veilige thread-safe accumulatie van stats
    private let statsQueue = DispatchQueue(label: "stats.queue")
    private var bytesReceived: Int = 0
    private var packetsReceived: Int = 0

    // Configuratie
    let streamPort: NWEndpoint.Port = 5000

    // ⚠️ VERANDER DIT NAAR HET LOKALE IP ADRES VAN JOUW PC! (Bijv. "192.168.1.50")
    let pcIPAddress = "192.168.0.189"

    func startListening() {
        guard !isStreaming else { return }
        isStreaming = true

        // 1. Forceer de Local Network Popup
        triggerLocalNetworkPrompt()

        // 2. Start de UDP Listener
        startNetworkListener()

        // 3. Start de statistieken monitor
        startStatsMonitoring()
    }

    func stopListening() {
        isStreaming = false
        listener?.cancel()
        listener = nil
        stopStatsMonitoring()

        decoder.reset()

        fps = 0
        bitrateMbps = 0
        latencyMs = 0
    }

    // MARK: - Apple Network Stack: Forceer iOS Popup
    private func triggerLocalNetworkPrompt() {
        let host = NWEndpoint.Host(pcIPAddress)
        let port = NWEndpoint.Port(rawValue: UInt16(streamPort.rawValue))!

        // Maak een uitgaande UDP connectie om de iOS privacy popup te forceren
        let connection = NWConnection(host: host, port: port, using: .udp)

        connection.stateUpdateHandler = { state in
            switch state {
            case .ready:
                print("✅ Ping verzonden. Local Network popup is getoond.")
                let pingData = "PING".data(using: .utf8)!
                connection.send(
                    content: pingData,
                    completion: .contentProcessed({ _ in
                        connection.cancel()  // Sluit de connectie, we hebben de popup nu geforceerd
                    }))
            case .failed(let error):
                print("❌ Local Network ping failed: \(error)")
                connection.cancel()
            default:
                break
            }
        }
        connection.start(queue: .global())
    }

    // MARK: - Apple Network Stack: UDP Listener
    private func startNetworkListener() {
        do {
            let params = NWParameters.udp
            params.allowLocalEndpointReuse = true
            listener = try NWListener(using: params, on: streamPort)

            listener?.stateUpdateHandler = { [weak self] state in
                switch state {
                case .ready:
                    print(
                        "✅ iPad NWListener luistert nu op UDP poort \(self?.streamPort.rawValue ?? 0)"
                    )
                case .failed(let error):
                    print("❌ NWListener gefaald: \(error)")
                    self?.stopListening()
                default:
                    break
                }
            }

            // Elke inkomende "connectie" (UDP packet stream) wordt hier afgehandeld
            listener?.newConnectionHandler = { [weak self] connection in
                guard let self = self else { return }
                connection.start(queue: .global())
                self.receiveData(on: connection)
            }

            listener?.start(queue: .global())
        } catch {
            print("❌ Fout bij aanmaken NWListener: \(error)")
        }
    }

    private func receiveData(on connection: NWConnection) {
        // Ontvang een compleet UDP message
        connection.receiveMessage { [weak self] data, context, isComplete, error in
            guard let self = self else { return }

            if let error = error {
                print("❌ Receive error: \(error)")
                connection.cancel()
                return
            }

            if let data = data, !data.isEmpty {
                // Update stats thread-safe
                self.statsQueue.sync {
                    self.bytesReceived += data.count
                    self.packetsReceived += 1
                }

                // Verwerk de ruwe H.264 data
                self.feedToDecoder(data: data)
            }

            // Blijf luisteren naar de volgende packets
            self.receiveData(on: connection)
        }
    }

    // MARK: - Veilige NAL Unit Parser
    // private func feedToDecoder(data: Data) {
    //     if data.isEmpty { return }

    //     let bytes = [UInt8](data)
    //     var i = 0
    //     var lastNaluStart = -1

    //     while i < bytes.count {
    //         var startCodeLength = 0
    //         // Check voor 4-byte startcode (00 00 00 01)
    //         if i + 3 < bytes.count && bytes[i] == 0 && bytes[i + 1] == 0 && bytes[i + 2] == 0
    //             && bytes[i + 3] == 1
    //         {
    //             startCodeLength = 4
    //         }
    //         // Check voor 3-byte startcode (00 00 01)
    //         else if i + 2 < bytes.count && bytes[i] == 0 && bytes[i + 1] == 0 && bytes[i + 2] == 1 {
    //             startCodeLength = 3
    //         }

    //         if startCodeLength > 0 {
    //             // Als we al een startcode hadden, hebben we nu het einde gevonden
    //             if lastNaluStart != -1 {
    //                 let payloadStart = lastNaluStart
    //                 let payloadEnd = i

    //                 if payloadEnd > payloadStart {
    //                     let payload = Data(bytes[payloadStart..<payloadEnd])
    //                     if payload.count > 0 {
    //                         let nalType = payload[0] & 0x1F
    //                         self.decoder.dispatch(payload, nalType: nalType)
    //                     }
    //                 }
    //             }

    //             // Update de startpositie voor de volgende NAL (na de startcode)
    //             lastNaluStart = i + startCodeLength
    //             i += startCodeLength
    //         } else {
    //             i += 1
    //         }
    //     }

    //     // Verwerk de allerlaatste NAL unit in de packet
    //     if lastNaluStart != -1 && lastNaluStart < bytes.count {
    //         let payload = Data(bytes[lastNaluStart..<bytes.count])
    //         if payload.count > 0 {
    //             let nalType = payload[0] & 0x1F
    //             self.decoder.dispatch(payload, nalType: nalType)
    //         }
    //     }
    // }

    // MARK: - Veilige NAL Unit Parser
    private func feedToDecoder(data: Data) {
        if data.isEmpty { return }

        let bytes = [UInt8](data)
        var i = 0
        var lastNaluStart = -1
        var foundStartCode = false

        while i < bytes.count {
            var startCodeLength = 0
            if i + 3 < bytes.count && bytes[i] == 0 && bytes[i + 1] == 0 && bytes[i + 2] == 0
                && bytes[i + 3] == 1
            {
                startCodeLength = 4
            } else if i + 2 < bytes.count && bytes[i] == 0 && bytes[i + 1] == 0 && bytes[i + 2] == 1
            {
                startCodeLength = 3
            }

            if startCodeLength > 0 {
                foundStartCode = true
                if lastNaluStart != -1 {
                    let payloadStart = lastNaluStart
                    let payloadEnd = i

                    if payloadEnd > payloadStart {
                        let payload = Data(bytes[payloadStart..<payloadEnd])
                        if payload.count > 0 {
                            let nalType = payload[0] & 0x1F
                            self.decoder.dispatch(payload, nalType: nalType)
                        }
                    }
                }

                lastNaluStart = i + startCodeLength
                i += startCodeLength
            } else {
                i += 1
            }
        }

        if foundStartCode {
            // Verwerk de laatste NAL unit in de packet
            if lastNaluStart != -1 && lastNaluStart < bytes.count {
                let payload = Data(bytes[lastNaluStart..<bytes.count])
                if payload.count > 0 {
                    let nalType = payload[0] & 0x1F
                    self.decoder.dispatch(payload, nalType: nalType)
                }
            }
        } else {
            // GEEN STARTCODE GEVONDEN!
            // De PC stuurt waarschijnlijk 1 NAL unit per UDP packet (raw payload)
            // We behandelen het hele packet direct als 1 NAL unit.
            let nalType = bytes[0] & 0x1F
            self.decoder.dispatch(data, nalType: nalType)
        }
    }

    // MARK: - Statistieken Monitor
    private func startStatsMonitoring() {
        statsQueue.sync {
            bytesReceived = 0
            packetsReceived = 0
        }

        let timer = DispatchSource.makeTimerSource(queue: .main)
        timer.schedule(deadline: .now() + 1, repeating: 1.0)
        timer.setEventHandler { [weak self] in
            guard let self = self else { return }

            self.statsQueue.sync {
                self.fps = Double(self.packetsReceived)
                self.bitrateMbps = (Double(self.bytesReceived) * 8) / 1000000.0

                self.bytesReceived = 0
                self.packetsReceived = 0
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
