import AVFoundation
import SwiftUI
import UIKit

struct ContentView: View {
    @StateObject private var viewModel = StreamEngine()
    @State private var isNotchVisible: Bool = true
    @State private var ipAddress: String? = nil

    var body: some View {
        ZStack(alignment: .leading) {
            // MARK: - Het Volledige Videobeeld & Debug
            ZStack {
                Color.black.ignoresSafeArea()

                VideoDisplayView(displayLayer: viewModel.decoder.displayLayer)
                    .ignoresSafeArea()

                if !viewModel.isStreaming {
                    VStack(spacing: 8) {
                        Text("Ready to Connect")
                            .font(.title2.weight(.bold))
                            .foregroundColor(.gray)

                        Text("IP: \(ipAddress ?? "Ophalen...")")
                            .font(.subheadline.monospaced())
                            .foregroundColor(.gray.opacity(0.8))
                    }
                    .allowsHitTesting(false)
                }
            }
            .onTapGesture {
                if viewModel.isStreaming {
                    withAnimation(.spring(response: 0.4, dampingFraction: 0.8)) {
                        isNotchVisible.toggle()
                    }
                }
            }
            .gesture(
                DragGesture(minimumDistance: 30)
                    .onEnded { value in
                        if viewModel.isStreaming {
                            if value.startLocation.x < 100 || value.translation.width > 50 {
                                withAnimation(.spring(response: 0.4, dampingFraction: 0.8)) {
                                    isNotchVisible = true
                                }
                            } else if isNotchVisible && value.translation.width < -50 {
                                withAnimation(.spring(response: 0.4, dampingFraction: 0.8)) {
                                    isNotchVisible = false
                                }
                            }
                        }
                    }
            )

            // MARK: - De Zwevende Notch
            if !viewModel.isStreaming || isNotchVisible {
                NotchView(viewModel: viewModel, isNotchVisible: $isNotchVisible)
                    .offset(x: viewModel.isStreaming && !isNotchVisible ? -200 : 16)
                    .transition(.move(edge: .leading).combined(with: .opacity))
            }
        }
        .statusBar(hidden: true)
        .persistentSystemOverlays(.hidden)
        .onAppear {
            refreshIPAddress()
        }
        .onChange(of: viewModel.isStreaming) { _, isStreaming in
            // Ververs het IP-adres zodra de stream stopt
            if !isStreaming {
                refreshIPAddress()
            }
        }
    }

    private func refreshIPAddress() {
        self.ipAddress = NetworkHelpers.getWiFiAddress()
    }
}

// MARK: - Helper voor IP ophalen
enum NetworkHelpers {
    static func getWiFiAddress() -> String? {
        var address: String?
        var ifaddr: UnsafeMutablePointer<ifaddrs>?

        guard getifaddrs(&ifaddr) == 0, let firstAddr = ifaddr else {
            return nil
        }

        for ptr in sequence(first: firstAddr, next: { $0.pointee.ifa_next }) {
            let interface = ptr.pointee
            let addrFamily = interface.ifa_addr.pointee.sa_family

            if addrFamily == UInt8(AF_INET) {
                let name = String(cString: interface.ifa_name)

                // en0 is standaard de Wi-Fi interface op iOS/iPadOS
                if name == "en0" {
                    var hostname = [CChar](repeating: 0, count: Int(NI_MAXHOST))
                    getnameinfo(
                        interface.ifa_addr,
                        socklen_t(interface.ifa_addr.pointee.sa_len),
                        &hostname,
                        socklen_t(hostname.count),
                        nil,
                        0,
                        NI_NUMERICHOST
                    )
                    address = String(cString: hostname)
                    break
                }
            }
        }

        freeifaddrs(ifaddr)
        return address
    }
}

// MARK: - UIViewRepresentable
struct VideoDisplayView: UIViewRepresentable {
    var displayLayer: AVSampleBufferDisplayLayer

    func makeUIView(context: Context) -> PlayerUIView {
        return PlayerUIView(displayLayer: displayLayer)
    }

    func updateUIView(_ uiView: PlayerUIView, context: Context) {}

    class PlayerUIView: UIView {
        let displayLayer: AVSampleBufferDisplayLayer

        init(displayLayer: AVSampleBufferDisplayLayer) {
            self.displayLayer = displayLayer
            super.init(frame: .zero)
            self.backgroundColor = .black
            self.layer.addSublayer(displayLayer)
        }

        required init?(coder: NSCoder) {
            fatalError("init(coder:) has not been implemented")
        }

        override func layoutSubviews() {
            super.layoutSubviews()
            displayLayer.frame = self.bounds
        }
    }
}

// MARK: - Notch UI Component
struct NotchView: View {
    @ObservedObject var viewModel: StreamEngine
    @Binding var isNotchVisible: Bool

    var body: some View {
        HStack(spacing: 16) {
            if viewModel.isStreaming && isNotchVisible {
                HStack(spacing: 12) {
                    StatPill(label: "FPS", value: String(format: "%.0f", viewModel.fps))
                    StatPill(label: "Mbps", value: String(format: "%.1f", viewModel.bitrateMbps))
                    StatPill(label: "MS", value: String(format: "%.0f", viewModel.latencyMs))
                }
                .transition(.scale.combined(with: .opacity))
            }

            HStack(spacing: 10) {
                if !viewModel.isStreaming {
                    Button(action: {
                        viewModel.startListening()
                        withAnimation(.spring(response: 0.4, dampingFraction: 0.8)) {
                            isNotchVisible = false
                        }
                    }) {
                        Image(systemName: "play.fill")
                            .foregroundColor(.white)
                            .frame(width: 30, height: 30)
                    }
                } else {
                    Button(action: {
                        viewModel.stopListening()
                        withAnimation(.spring(response: 0.4, dampingFraction: 0.8)) {
                            isNotchVisible = true
                        }
                    }) {
                        Image(systemName: "stop.fill")
                            .foregroundColor(.red)
                            .frame(width: 30, height: 30)
                    }
                }
            }
        }
        .padding(.vertical, 10)
        .padding(.horizontal, 16)
        .background(
            Capsule()
                .fill(.ultraThinMaterial)
                .overlay(
                    Capsule().stroke(Color.white.opacity(0.2), lineWidth: 1)
                )
                .shadow(color: .black.opacity(0.5), radius: 10, x: 0, y: 5)
        )
    }
}

// MARK: - Klein statistiek pilletje
struct StatPill: View {
    var label: String
    var value: String

    var body: some View {
        VStack(spacing: 0) {
            Text(value)
                .font(.system(size: 14, weight: .bold, design: .rounded))
                .foregroundColor(.white)
            Text(label)
                .font(.system(size: 8, weight: .medium, design: .rounded))
                .foregroundColor(.gray)
        }
    }
}
