import SwiftUI

/// Center column: filterable key table.
struct KeyListView: View {
    let keys: [KeyRow]
    @Binding var selectedKey: KeyRow?

    var body: some View {
        if keys.isEmpty {
            ContentUnavailableView(
                "No File Selected",
                systemImage: "doc",
                description: Text("Choose a file from the sidebar.")
            )
        } else {
            Table(keys, selection: Binding(
                get: { selectedKey?.key },
                set: { newSel in
                    selectedKey = keys.first { $0.key == newSel }
                }
            )) {
                TableColumn("Key") { row in
                    HStack(spacing: 6) {
                        if !row.missingLocales.isEmpty {
                            Image(systemName: "exclamationmark.triangle.fill")
                                .foregroundStyle(.orange)
                                .help("Missing in \(row.missingLocales.count) locale(s)")
                        }
                        Text(row.key)
                            .font(.system(.body, design: .monospaced))
                            .lineLimit(1)
                            .truncationMode(.tail)
                    }
                    .help(row.key)
                }
                TableColumn("Status") { row in
                    let total = row.translations.count
                    let filled = row.translations.values.filter { $0 != nil }.count
                    Text("\(filled)/\(total)")
                        .font(.caption)
                        .foregroundStyle(filled == total ? .green : .secondary)
                }
            }
        }
    }
}
