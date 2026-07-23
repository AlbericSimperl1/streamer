import AVFoundation
import SwiftUI
import UIKit

struct ContentView: View {
    @StateObject private var viewModel = StreamEngine()
    @State private var isNotchVisible: Bool = true

    var body: some View {
        ZStack(alignment: .leading) {
            // MARK: - Het Volledige Videobeeld & Debug
            ZStack {
                Color.black.ignoresSafeArea()

                VideoDisplayView(displayLayer: viewModel.decoder.displayLayer)
                    .ignoresSafeArea()

                if !viewModel.isStreaming {
                    Text("Ready to Connect")
                        .font(.title2)
                        .foregroundColor(.gray)
                        .allowsHitTesting(false)
                } else {
                    Text(viewModel.decoderDebug)
                        .font(.system(size: 14, weight: .bold))
                        .foregroundColor(.green)
                        .padding(8)
                        .background(Color.black.opacity(0.6))
                        .cornerRadius(8)
                        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottom)
                        .padding(.bottom, 40)
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
        .persistentSystemOverlays(.hidden)
    }
}

// // MARK: - UIViewRepresentable
// struct VideoDisplayView: UIViewRepresentable {
//     var displayLayer: AVSampleBufferDisplayLayer

//     func makeUIView(context: Context) -> PlayerUIView {
//         return PlayerUIView(displayLayer: displayLayer)
//     }

//     func updateUIView(_ uiView: PlayerUIView, context: Context) {}

//     class PlayerUIView: UIView {
//         let displayLayer: AVSampleBufferDisplayLayer

//         init(displayLayer: AVSampleBufferDisplayLayer) {
//             self.displayLayer = displayLayer
//             super.init(frame: .zero)
//             self.layer.addSublayer(displayLayer)
//         }

//         required init?(coder: NSCoder) {
//             fatalError("init(coder:) has not been implemented")
//         }

//         override func layoutSubviews() {
//             super.layoutSubviews()
//             displayLayer.frame = self.bounds
//         }
//     }
// }

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
            // TIJDELIJK: Maak de achtergrond rood om te zien of de view echt op het scherm staat!
            self.backgroundColor = .black
            self.layer.addSublayer(displayLayer)
        }

        required init?(coder: NSCoder) {
            fatalError("init(coder:) has not been implemented")
        }

        override func layoutSubviews() {
            super.layoutSubviews()
            // Forceer de layer altijd exact zo groot als de view zelf
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
