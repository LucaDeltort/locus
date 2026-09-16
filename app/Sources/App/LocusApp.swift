import SwiftUI
import AppKit

@main
struct LocusApp: App {
    init() {
        // Set the app icon at runtime.
        if let icon = NSImage(named: "AppIcon") {
            NSApplication.shared.applicationIconImage = icon
        } else {
            let iconPath = Bundle.main.path(forResource: "AppIcon", ofType: "icns")
            if let path = iconPath, let img = NSImage(contentsOfFile: path) {
                NSApplication.shared.applicationIconImage = img
            }
        }
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
                .frame(minWidth: 900, minHeight: 500)
        }
        .windowToolbarStyle(.unified)
    }
}
