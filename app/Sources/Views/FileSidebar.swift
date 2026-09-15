import SwiftUI

/// Sidebar: list of localization files.
struct FileSidebar: View {
    let files: [FileInfo]
    @Binding var selectedFile: String?
    let isLoading: Bool

    var body: some View {
        List(selection: $selectedFile) {
            if !files.isEmpty {
                Section("Files") {
                    ForEach(files, id: \.name) { file in
                        HStack {
                            Image(systemName: file.format == "xcstrings"
                                  ? "doc.text.fill" : "doc.text")
                                .foregroundStyle(LocusTheme.violet)
                            Text(file.name)
                            Spacer()
                            Text("\(file.locales.count)")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                        .tag(file.name)
                    }
                }
            } else if isLoading {
                HStack(spacing: 8) {
                    ProgressView().controlSize(.small)
                    Text("Loading…")
                }
                .frame(maxWidth: .infinity, alignment: .center)
                .padding(.top, 40)
            } else {
                ContentUnavailableView(
                    "No Project Open",
                    systemImage: "folder.badge.plus",
                    description: Text("Click Open to load a localization project.")
                )
            }
        }
        .navigationTitle("Locus")
        .frame(minWidth: 200)
    }
}
