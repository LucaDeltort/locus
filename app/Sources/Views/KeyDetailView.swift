import SwiftUI

/// Right panel: edit all locale values for a single key.
///
/// Each TextField writes directly to the Rust model via the ViewModel
/// callback — no local edit buffer, no per-row save button.
struct KeyDetailView: View {
    let key: KeyRow
    let fileName: String
    let onSetValue: (String, String) -> Void  // (lang, value)

    @State private var showCopied = false

    /// Sorted locale list for stable display.
    private var sortedLocales: [String] {
        key.translations.keys.sorted()
    }

    /// First non-empty translation value, used as copy-from-source.
    private var referenceValue: String? {
        for lang in sortedLocales {
            if let val = key.translations[lang], let v = val, !v.isEmpty {
                return v
            }
        }
        return nil
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                header
                Divider()
                ForEach(sortedLocales, id: \.self) { lang in
                    localeRow(lang)
                }
                if showCopied {
                    Text("Value copied to clipboard.")
                        .font(.caption)
                        .foregroundStyle(LocusTheme.violet)
                        .transition(.opacity)
                }
            }
            .padding(20)
        }
        .navigationTitle("Translations")
    }

    // MARK: - Header

    @ViewBuilder
    private var header: some View {
        HStack {
            Text(key.key)
                .font(.system(.title3, design: .monospaced))
                .fontWeight(.semibold)
                .textSelection(.enabled)
            Spacer()
            if !key.missingLocales.isEmpty {
                Label("\(key.missingLocales.count) missing",
                      systemImage: "exclamationmark.triangle.fill")
                    .font(.caption)
                    .foregroundStyle(.orange)
            }
        }
        .padding(.bottom, 4)
    }

    // MARK: - Locale row

    @ViewBuilder
    private func localeRow(_ lang: String) -> some View {
        let currentValue = key.translations[lang] ?? nil
        let isMissing = key.missingLocales.contains(lang)

        HStack(alignment: .top, spacing: 12) {
            Text(lang)
                .font(.system(.body, design: .monospaced))
                .frame(width: 50, alignment: .leading)
                .foregroundStyle(isMissing ? .orange : .secondary)

            TextField(
                "Missing…",
                text: Binding(
                    get: { currentValue ?? "" },
                    set: { newVal in onSetValue(lang, newVal) }
                )
            )
            .textFieldStyle(.roundedBorder)

            if isMissing, let refVal = referenceValue {
                Button {
                    #if canImport(AppKit)
                    NSPasteboard.general.clearContents()
                    NSPasteboard.general.setString(refVal, forType: .string)
                    #endif
                    onSetValue(lang, refVal)
                    withAnimation { showCopied = true }
                    DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) {
                        withAnimation { showCopied = false }
                    }
                } label: {
                    Image(systemName: "doc.on.clipboard")
                        .help("Copy from source")
                }
                .buttonStyle(.borderless)
            }
        }
        .padding(.vertical, 2)
    }
}
