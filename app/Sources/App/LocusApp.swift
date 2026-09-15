import SwiftUI
import AppKit

@main
struct LocusApp: App {
    init() {
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
                .frame(minWidth: 900, minHeight: 500)
        }
        .windowToolbarStyle(.unified)
    }
}
