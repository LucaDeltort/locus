import Foundation
import Observation

/// Central state for the project, bridging the Rust core and SwiftUI views.
///
/// Caches key data to avoid repeated FFI round-trips on every view re-render.
@Observable
final class ProjectViewModel {
    /// The loaded Rust project handle, or nil when no project is open.
    private(set) var project: LocusProject?

    /// Currently selected logical file name (sidebar).
    var selectedFile: String? {
        didSet { invalidateKeys() }
    }

    /// Currently selected key row (center table).
    var selectedKey: KeyRow?

    // MARK: - Search

    var searchText = "" {
        didSet { invalidateFilteredKeys() }
    }

    enum SearchMode: String, CaseIterable, Identifiable {
        case key = "Key"
        case text = "Value"
        var id: String { rawValue }
    }

    var searchMode: SearchMode = .key {
        didSet { invalidateFilteredKeys() }
    }

    // MARK: - State flags

    private(set) var isLoading = false
    private(set) var unsavedChanges = false
    var errorMessage: String?
    var showError = false

    // MARK: - Cached data

    /// All files in the project (sidebar). Nil = not loaded yet.
    private(set) var files: [FileInfo] = []

    /// All keys for the selected file, cached after first load.
    private var allKeysCache: [KeyRow] = []

    /// Filtered keys based on current search text + mode.
    private(set) var filteredKeys: [KeyRow] = []

    // MARK: - Loading

    /// Load a project from disk by path.
    func load(path: String) {
        isLoading = true
        defer { isLoading = false }
        do {
            let proj = try loadProject(path: path)
            project = proj
            files = proj.files()
            ProjectPersistence.lastPath = path
            selectedFile = nil
            selectedKey = nil
            unsavedChanges = false
        } catch {
            errorMessage = "\(error)"
            showError = true
        }
    }

    /// Restore the last-opened project from UserDefaults.
    func restoreLastProject() {
        guard project == nil, !isLoading, ProjectPersistence.exists else { return }
        if let path = ProjectPersistence.lastPath {
            load(path: path)
        }
    }

    // MARK: - Keys

    /// Ensure the cache for the selected file is up to date.
    private func invalidateKeys() {
        guard let project, let selectedFile else {
            allKeysCache = []
            filteredKeys = []
            return
        }
        allKeysCache = project.keys(fileName: selectedFile)
        applyFilter()
    }

    private func invalidateFilteredKeys() {
        applyFilter()
    }

    private func applyFilter() {
        guard !searchText.isEmpty else {
            filteredKeys = allKeysCache
            return
        }
        switch searchMode {
        case .key:
            filteredKeys = allKeysCache.filter { $0.key.contains(searchText) }
        case .text:
            filteredKeys = allKeysCache.filter { row in
                row.translations.values.contains { val in
                    val?.contains(searchText) ?? false
                }
            }
        }
    }

    // MARK: - Editing

    /// Set a translation value for a specific locale.
    func setValue(file: String, key: String, lang: String, value: String) throws {
        guard let project else { return }
        try project.setValue(file: file, key: key, lang: lang, value: value)
        unsavedChanges = true
        // Refresh the cache so the UI shows updated state.
        invalidateKeys()
        // Update selectedKey to reflect the new value in the detail view.
        if let idx = filteredKeys.firstIndex(where: { $0.key == key }) {
            selectedKey = filteredKeys[idx]
        }
    }

    /// Save all changes to disk atomically.
    func save() {
        guard let project else { return }
        do {
            try project.save(backup: false)
            unsavedChanges = false
        } catch {
            errorMessage = "\(error)"
            showError = true
        }
    }
}
