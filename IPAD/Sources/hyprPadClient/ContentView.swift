import AVFoundation
import SwiftUI
import UIKit

struct ContentView: View {
    @StateObject private var viewModel = StreamEngine()
    @State private var isNotchVisible: Bool = true

    var body: some View {
        ZStack(alignment: .leading) {
            // MARK: - De Video Layer
            ZStack {
                Color.black.ignoresSafeArea()

                // Jouw decoder's display layer
                VideoDisplayView(displayLayer: viewModel.decoder.displayLayer)
                    .ignoresSafeArea()

                if !viewModel.isStreaming {
                    Text("Ready to Connect")
                        .font(.title2)
                        .foregroundColor(.gray)
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

            // MARK: - De Notch
            if !viewModel.isStreaming || isNotchVisible {
                NotchView(viewModel: viewModel, isNotchVisible: $isNotchVisible)
                    .offset(x: viewModel.isStreaming && !isNotchVisible ? -200 : 16)
                    .transition(.move(edge: .leading).combined(with: .opacity))
            }
        }
        .persistentSystemOverlays(.hidden)
    }
}

// MARK: - UIViewRepresentable om de AVSampleBufferDisplayLayer in SwiftUI te tonen
// struct VideoDisplayView: UIViewRepresentable {
//     var displayLayer: AVSampleBufferDisplayLayer

//     func makeUIView(context: Context) -> UIView {
//         let view = UIView()
//         view.backgroundColor = .black
//         view.layer.addSublayer(displayLayer)
//         return view
//     }

//     func updateUIView(_ uiView: UIView, context: Context) {
//         // Zorg dat de videolayer altijd precies de grootte van het scherm aanneemt
//         displayLayer.frame = uiView.bounds
//     }
// }

// MARK: - UIViewRepresentable om de AVSampleBufferDisplayLayer in SwiftUI te tonen
struct VideoDisplayView: UIViewRepresentable {
    var displayLayer: AVSampleBufferDisplayLayer

    func makeUIView(context: Context) -> PlayerUIView {
        return PlayerUIView(displayLayer: displayLayer)
    }

    func updateUIView(_ uiView: PlayerUIView, context: Context) {
        // Update wordt aangeroepen door SwiftUI
    }

    // Een custom UIView die de layer altijd perfect uitrekt
    class PlayerUIView: UIView {
        let displayLayer: AVSampleBufferDisplayLayer

        init(displayLayer: AVSampleBufferDisplayLayer) {
            self.displayLayer = displayLayer
            super.init(frame: .zero)
            self.layer.addSublayer(displayLayer)
        }

        required init?(coder: NSCoder) {
            fatalError("init(coder:) has not been implemented")
        }

        override func layoutSubviews() {
            super.layoutSubviews()
            // Cruciaal: Zorg dat de video always het hele scherm vult
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
