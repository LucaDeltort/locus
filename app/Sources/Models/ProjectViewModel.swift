import Foundation
import Observation

/// Central state for the project, bridging the Rust core and SwiftUI views.
///
/// Caches key data to avoid repeated FFI round-trips on every view re-render.
@Observable
final class ProjectViewModel {
    /// The loaded Rust project handle, or nil when no project is open.
    private(set) var project: LocusProject?

    /// Path of the currently open file/folder, for display in the toolbar.
    private(set) var currentFilePath: String?

    /// Source language of the currently selected file (e.g. "en").
    private(set) var sourceLanguage: String = "en"

    /// Currently selected logical file name (auto-selected on load).
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

    // MARK: - Locale filter

    /// Set of locale codes to show in the detail view. Empty = show all.
    /// The source language is always shown.
    var visibleLocales: Set<String> = [] {
        didSet { applyFilter() }
    }

    /// All available locales for the current file.
    private(set) var allLocales: [String] = []

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

    /// Locales to display in the detail view, respecting the filter.
    /// Source language is always first.
    var displayedLocales: [String] {
        guard !visibleLocales.isEmpty else { return allLocales }
        var result = allLocales.filter { visibleLocales.contains($0) }
        // Ensure source language is always present and first
        if let src = allLocales.first(where: { $0 == sourceLanguage }) {
            if !result.contains(src) { result.insert(src, at: 0) }
        }
        return result
    }

    // MARK: - Loading

    /// Load a project from disk by path.
    func load(path: String) {
        isLoading = true
        defer { isLoading = false }
        do {
            let proj = try loadProject(path: path)
            project = proj
            files = proj.files()
            currentFilePath = path
            ProjectPersistence.lastPath = path
            // Auto-select the first (or only) file.
            selectedFile = files.first?.name
            if let sf = files.first {
                sourceLanguage = sf.sourceLanguage
                allLocales = sf.locales
            }
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

    // MARK: - Undo

    /// Undo stack: each entry is (file, key, lang, originalValue).
    /// Consecutive edits to the same field are grouped into one entry.
    private var undoStack: [(file: String, key: String, lang: String, originalValue: String?)] = []

    /// Maximum number of undo entries retained. Older entries are evicted
    /// once the cap is reached (FIFO), bounding memory on long editing sessions.
    private let maxUndoEntries = 50

    /// Tracks the last field edited, for grouping consecutive keystrokes.
    private var lastEditedField: (file: String, key: String, lang: String)?

    var canUndo: Bool { !undoStack.isEmpty }

    /// Push an entry onto the undo stack, evicting the oldest if over cap.
    private func pushUndo(_ entry: (file: String, key: String, lang: String, originalValue: String?)) {
        undoStack.append(entry)
        if undoStack.count > maxUndoEntries {
            undoStack.removeFirst(undoStack.count - maxUndoEntries)
        }
    }

    /// Undo the last edit.
    func undo() {
        guard let last = undoStack.popLast(), let project else { return }
        do {
            if last.lang.isEmpty {
                // Undo addKey by deleting the key.
                try project.deleteKey(file: last.file, key: last.key)
                invalidateKeys()
            } else {
                // Undo setValue by restoring the original value.
                try project.setValue(file: last.file, key: last.key, lang: last.lang, value: last.originalValue ?? "")

                // Update selectedKey in-place (no FFI reload).
                if var sk = selectedKey {
                    sk.translations[last.lang] = last.originalValue
                    if let val = last.originalValue, !val.isEmpty {
                        sk.missingLocales.removeAll { $0 == last.lang }
                    } else if !sk.missingLocales.contains(last.lang) {
                        sk.missingLocales.append(last.lang)
                    }
                    selectedKey = sk
                }

                // Update cache in-place.
                if let idx = allKeysCache.firstIndex(where: { $0.key == last.key }) {
                    var row = allKeysCache[idx]
                    row.translations[last.lang] = last.originalValue
                    if let val = last.originalValue, !val.isEmpty {
                        row.missingLocales.removeAll { $0 == last.lang }
                    } else if !row.missingLocales.contains(last.lang) {
                        row.missingLocales.append(last.lang)
                    }
                    allKeysCache[idx] = row
                }
                applyFilter()
            }
            unsavedChanges = true
            lastEditedField = nil
        } catch {
            errorMessage = "\(error)"
            showError = true
        }
    }

    // MARK: - Editing

    /// Set a translation value for a specific locale.
    /// - Parameter isCopy: when true (copy-from-source button), always
    ///   pushes a new undo entry, even if the same field was just edited.
    func setValue(file: String, key: String, lang: String, value: String, isCopy: Bool = false) throws {
        guard let project else { return }

        // Group consecutive edits to the same field into one undo entry,
        // unless this is a copy-from-source action (which should be its own undo step).
        if !isCopy, let last = lastEditedField, last.file == file, last.key == key, last.lang == lang {
            // Same field: don't push again, the original value is already on the stack.
        } else {
            // New field (or copy): push the original value onto the undo stack.
            let oldValue = selectedKey?.translations[lang] ?? nil
            pushUndo((file: file, key: key, lang: lang, originalValue: oldValue))
            lastEditedField = (file: file, key: key, lang: lang)
        }

        try project.setValue(file: file, key: key, lang: lang, value: value)
        unsavedChanges = true

        // Update the selectedKey in-place so the detail view reflects
        // the new value without a full FFI reload (which would lose focus).
        if var sk = selectedKey {
            sk.translations[lang] = value.isEmpty ? nil : value
            if value.isEmpty && !sk.missingLocales.contains(lang) {
                sk.missingLocales.append(lang)
            } else if !value.isEmpty {
                sk.missingLocales.removeAll { $0 == lang }
            }
            selectedKey = sk
        }

        // Update the cache in-place too, so the table status badge updates.
        if let idx = allKeysCache.firstIndex(where: { $0.key == key }) {
            var row = allKeysCache[idx]
            row.translations[lang] = value.isEmpty ? nil : value
            if value.isEmpty && !row.missingLocales.contains(lang) {
                row.missingLocales.append(lang)
            } else if !value.isEmpty {
                row.missingLocales.removeAll { $0 == lang }
            }
            allKeysCache[idx] = row
        }
        applyFilter()
    }

    /// Add a new key with a base-locale value.
    func addKey(file: String, key: String, baseLang: String, baseValue: String) throws {
        guard let project else { return }
        try project.addKey(file: file, key: key, baseLang: baseLang, baseValue: baseValue)
        unsavedChanges = true
        // Undo for addKey = delete the key.
        pushUndo((file: file, key: key, lang: "", originalValue: nil))
        lastEditedField = nil
        invalidateKeys()
        if let idx = filteredKeys.firstIndex(where: { $0.key == key }) {
            selectedKey = filteredKeys[idx]
        }
    }

    /// Delete a key from all locales.
    func deleteKey(file: String, key: String) throws {
        guard let project else { return }
        try project.deleteKey(file: file, key: key)
        unsavedChanges = true
        // No undo for delete (irreversible — would need to store all values).
        lastEditedField = nil
        if selectedKey?.key == key {
            selectedKey = nil
        }
        invalidateKeys()
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
