import SwiftUI
import AppKit

/// Main window: 2-column layout (key list | translation editor).
/// No sidebar — we open a single file at a time.
struct ContentView: View {
    @State private var viewModel = ProjectViewModel()
    @State private var showAddKeySheet = false
    @State private var showDeleteConfirm = false
    @State private var showLocaleFilter = false

    var body: some View {
        NavigationSplitView {
            // --- Center: key list ---
            KeyListView(
                keys: viewModel.filteredKeys,
                selectedKey: $viewModel.selectedKey
            )
            .frame(minWidth: 300)
        } detail: {
            // --- Detail: translation editor ---
            if let key = viewModel.selectedKey {
                KeyDetailView(
                    key: key,
                    fileName: viewModel.selectedFile ?? "",
                    sourceLanguage: viewModel.sourceLanguage,
                    displayedLocales: viewModel.displayedLocales,
                    onSetValue: { lang, value, isCopy in
                        do {
                            try viewModel.setValue(
                                file: viewModel.selectedFile ?? "",
                                key: key.key,
                                lang: lang,
                                value: value,
                                isCopy: isCopy
                            )
                        } catch {
                            viewModel.errorMessage = "\(error)"
                            viewModel.showError = true
                        }
                    }
                )
            } else {
                ContentUnavailableView(
                    "Select a Key",
                    systemImage: "text.alignleft",
                    description: Text("Pick a key from the list to edit its translations.")
                )
            }
        }
        .navigationTitle("Locus")
        .toolbar { toolbarContent }
        .tint(LocusTheme.violet)
        .searchable(text: $viewModel.searchText, prompt: "Search \(viewModel.searchMode == .key ? "keys" : "values")…")
        .alert("Error", isPresented: $viewModel.showError) {
            Button("OK", role: .cancel) {}
        } message: {
            Text(viewModel.errorMessage ?? "")
        }
        .alert("Delete Key", isPresented: $showDeleteConfirm) {
            Button("Delete", role: .destructive) {
                if let key = viewModel.selectedKey?.key,
                   let file = viewModel.selectedFile {
                    do {
                        try viewModel.deleteKey(file: file, key: key)
                    } catch {
                        viewModel.errorMessage = "\(error)"
                        viewModel.showError = true
                    }
                }
            }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("Are you sure you want to delete \"\(viewModel.selectedKey?.key ?? "")\"? This cannot be undone.")
        }
        .sheet(isPresented: $showAddKeySheet) {
            AddKeySheet(
                files: viewModel.files,
                selectedFile: viewModel.selectedFile,
                onAdd: { file, key, lang, value in
                    do {
                        try viewModel.addKey(file: file, key: key, baseLang: lang, baseValue: value)
                        showAddKeySheet = false
                    } catch {
                        viewModel.errorMessage = "\(error)"
                        viewModel.showError = true
                    }
                }
            )
        }
        .onAppear { viewModel.restoreLastProject() }
        .onOpenURL { url in
            viewModel.load(path: url.path)
        }
        // Status bar at the bottom showing the current file path.
        // Truncated in the middle, but full path on hover via .help().
        .overlay(alignment: .bottom) {
            if let path = viewModel.currentFilePath {
                HStack(spacing: 6) {
                    Image(systemName: "folder")
                        .font(.caption2)
                    Text(path)
                        .font(.system(.caption, design: .monospaced))
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
                .foregroundStyle(.secondary)
                .padding(.horizontal, 12)
                .padding(.vertical, 4)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(.bar)
                .help(path)
            }
        }
    }

    // MARK: - Toolbar

    @ToolbarContentBuilder
    private var toolbarContent: some ToolbarContent {
        ToolbarItemGroup(placement: .primaryAction) {
            Button("Open…") { openFolder() }
                .keyboardShortcut("o")

            if viewModel.project != nil {
                Button("Save") { viewModel.save() }
                    .keyboardShortcut("s")
                    .disabled(!viewModel.unsavedChanges)

                Button {
                    viewModel.undo()
                } label: {
                    Label("Undo", systemImage: "arrow.uturn.backward")
                }
                .keyboardShortcut("z", modifiers: [.command])
                .disabled(!viewModel.canUndo)

                Button {
                    showAddKeySheet = true
                } label: {
                    Label("Add Key", systemImage: "plus")
                }
                .keyboardShortcut("n", modifiers: [.command])

                Button {
                    showDeleteConfirm = true
                } label: {
                    Label("Delete Key", systemImage: "minus")
                }
                .keyboardShortcut(.delete, modifiers: [.command])
                .disabled(viewModel.selectedKey == nil)

                Picker("Search", selection: $viewModel.searchMode) {
                    ForEach(ProjectViewModel.SearchMode.allCases) { mode in
                        Text(mode.rawValue).tag(mode)
                    }
                }
                .pickerStyle(.segmented)
                .help("Search by key or by value")

                // Locale filter — popover stays open while toggling
                Button {
                    showLocaleFilter = true
                } label: {
                    Label(
                        viewModel.visibleLocales.isEmpty
                            ? "All Locales"
                            : "\(viewModel.visibleLocales.count) Locale\(viewModel.visibleLocales.count == 1 ? "" : "s")",
                        systemImage: "globe"
                    )
                }
                .help("Filter which locales to show in the detail view")
                .popover(isPresented: $showLocaleFilter, arrowEdge: .bottom) {
                    LocaleFilterPopover(
                        allLocales: viewModel.allLocales,
                        visibleLocales: $viewModel.visibleLocales
                    )
                }
            }
        }
    }

    // MARK: - File picker

    private func openFolder() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = true
        panel.allowsMultipleSelection = false
        panel.allowedContentTypes = []
        panel.message = "Choose a localization project folder or .xcstrings file"
        let response = panel.runModal()
        guard response == .OK, let url = panel.url else { return }
        viewModel.load(path: url.path)
    }
}
