import Speakrs
import SwiftUI

@main
struct DiarizeFileApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}

struct ContentView: View {
    @StateObject private var model = SampleDiarizationModel()

    var body: some View {
        NavigationView {
            List {
                Section {
                    Button {
                        model.run()
                    } label: {
                        Label(model.buttonTitle, systemImage: "waveform")
                    }
                    .disabled(model.isRunning)
                }

                Section("Status") {
                    Text(model.statusText)
                        .font(.body.monospaced())
                        .textSelection(.enabled)
                }
            }
            .navigationTitle("Speakrs")
        }
        .navigationViewStyle(.stack)
    }
}

@MainActor
final class SampleDiarizationModel: ObservableObject {
    @Published private var state = SampleState.idle

    var buttonTitle: String {
        switch state {
        case .running:
            "Running"
        default:
            "Diarize sample file"
        }
    }

    var isRunning: Bool {
        if case .running = state {
            return true
        }
        return false
    }

    var statusText: String {
        switch state {
        case .idle:
            "Ready"
        case .running:
            "Preparing models and running diarization..."
        case .finished(let report):
            report.description
        case .failed(let message):
            message
        }
    }

    func run() {
        state = .running

        Task {
            do {
                let report = try await Task.detached(priority: .userInitiated) {
                    try SampleDiarization.run()
                }.value
                state = .finished(report)
            } catch {
                state = .failed(String(describing: error))
            }
        }
    }
}

enum SampleState {
    case idle
    case running
    case finished(SampleReport)
    case failed(String)
}

struct SampleReport: Sendable {
    let segmentCount: Int
    let duration: Double
    let mode: ExecutionMode
    let modelRevision: String
    let rttm: String

    var description: String {
        """
        Mode: \(mode)
        Duration: \(duration)
        Segments: \(segmentCount)
        Model revision: \(modelRevision)

        \(rttm)
        """
    }
}

enum SampleDiarization {
    static func run() throws -> SampleReport {
        let mode = ExecutionMode.coreMl
        let audioURL = try bundledAudioURL()
        let modelDir = bundledModelDir()
        let prepared = try prepareModels(mode: mode, cacheDir: nil, modelDir: modelDir)
        let pipelineConfig = defaultPipelineConfig(mode: mode)
        let runtimeConfig = defaultRuntimeConfig()
        let pipeline = try SpeakrsPipeline.fromPrepared(
            prepared: prepared,
            mode: mode,
            pipelineConfig: pipelineConfig,
            runtimeConfig: runtimeConfig
        )
        let result = try pipeline.diarizeFile(
            path: audioURL.path,
            fileId: "test_short",
            pipelineConfig: pipelineConfig,
            cancelToken: nil
        )

        return SampleReport(
            segmentCount: result.segments.count,
            duration: result.duration,
            mode: result.mode,
            modelRevision: result.modelRevision,
            rttm: result.rttm
        )
    }

    private static func bundledAudioURL() throws -> URL {
        guard let url = Bundle.main.url(forResource: "test_short", withExtension: "wav") else {
            throw SampleDiarizationError.missingAudioFixture
        }
        return url
    }

    private static func bundledModelDir() -> String? {
        guard let url = Bundle.main.resourceURL?.appendingPathComponent("Models", isDirectory: true),
              FileManager.default.fileExists(atPath: url.path)
        else {
            return nil
        }
        return url.path
    }
}

enum SampleDiarizationError: Error, CustomStringConvertible {
    case missingAudioFixture

    var description: String {
        switch self {
        case .missingAudioFixture:
            "The bundled test_short.wav fixture is missing"
        }
    }
}
