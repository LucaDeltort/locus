import SwiftUI
import AppKit

/// Main window: NavigationSplitView with sidebar (files), content (keys),
/// and detail (translations editor).
struct ContentView: View {
    @State private var viewModel = ProjectViewModel()

    var body: some View {
        NavigationSplitView {
            FileSidebar(
                files: viewModel.files,
                selectedFile: $viewModel.selectedFile,
                isLoading: viewModel.isLoading
            )
        } content: {
            KeyListView(
                keys: viewModel.filteredKeys,
                selectedKey: $viewModel.selectedKey
            )
            .frame(minWidth: 300)
        } detail: {
            if let key = viewModel.selectedKey {
                KeyDetailView(
                    key: key,
                    fileName: viewModel.selectedFile ?? "",
                    onSetValue: { lang, value in
                        do {
                            try viewModel.setValue(
                                file: viewModel.selectedFile ?? "",
                                key: key.key,
                                lang: lang,
                                value: value
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
        .toolbar { toolbarContent }
        .tint(LocusTheme.violet)
        .searchable(text: $viewModel.searchText, prompt: "Search \(viewModel.searchMode == .key ? "keys" : "values")…")
        .alert("Error", isPresented: $viewModel.showError) {
            Button("OK", role: .cancel) {}
        } message: {
            Text(viewModel.errorMessage ?? "")
        }
        .onAppear { viewModel.restoreLastProject() }
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

                Picker("Search", selection: $viewModel.searchMode) {
                    ForEach(ProjectViewModel.SearchMode.allCases) { mode in
                        Text(mode.rawValue).tag(mode)
                    }
                }
                .pickerStyle(.segmented)
                .help("Search by key or by value")
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
