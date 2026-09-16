import SwiftUI

/// Wrapper around KeyRow that exposes sortable properties for Table.
/// UniFFI-generated types can't conform to protocols, so we wrap them here.
struct SortableKeyRow: Identifiable {
    let row: KeyRow

    var id: String { row.key }
    var key: String { row.key }
    var filledCount: Int {
        row.translations.values.filter { $0 != nil }.count
    }
}

/// Center column: filterable key table with native column sorting.
struct KeyListView: View {
    let keys: [KeyRow]
    @Binding var selectedKey: KeyRow?

    @State private var sortOrder = [KeyPathComparator(\SortableKeyRow.key)]

    private var sortedRows: [SortableKeyRow] {
        keys.map { SortableKeyRow(row: $0) }.sorted(using: sortOrder)
    }

    var body: some View {
        if keys.isEmpty {
            ContentUnavailableView(
                "No File Selected",
                systemImage: "doc",
                description: Text("Choose a file from the sidebar.")
            )
        } else {
            Table(
                sortedRows,
                selection: Binding(
                    get: { selectedKey?.key },
                    set: { newSel in
                        selectedKey = sortedRows.first { $0.row.key == newSel }?.row
                    }
                ),
                sortOrder: $sortOrder
            ) {
                TableColumn("Key", value: \.key) { item in
                    HStack(spacing: 6) {
                        if !item.row.missingLocales.isEmpty {
                            Image(systemName: "exclamationmark.triangle.fill")
                                .foregroundStyle(.orange)
                                .help("Missing in \(item.row.missingLocales.count) locale(s)")
                        }
                        Text(item.row.key)
                            .font(.system(.body, design: .monospaced))
                            .lineLimit(1)
                            .truncationMode(.tail)
                    }
                    .help(item.row.key)
                }

                TableColumn("Status", value: \.filledCount) { item in
                    let total = item.row.translations.count
                    let filled = item.row.translations.values.filter { $0 != nil }.count
                    Text("\(filled)/\(total)")
                        .font(.caption)
                        .foregroundStyle(filled == total ? .green : .secondary)
                }
            }
        }
    }
}
