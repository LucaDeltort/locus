import SwiftUI

/// Persists the last-opened project path across app launches.
/// Stored in UserDefaults so it survives restarts.
enum ProjectPersistence {
    private static let key = "locus.lastProjectPath"

    /// The last project path, if any.
    static var lastPath: String? {
        get { UserDefaults.standard.string(forKey: key) }
        set { UserDefaults.standard.set(newValue, forKey: key) }
    }

    /// Whether a persisted path exists and is still valid on disk.
    static var exists: Bool {
        guard let path = lastPath else { return false }
        return FileManager.default.fileExists(atPath: path)
    }
}
