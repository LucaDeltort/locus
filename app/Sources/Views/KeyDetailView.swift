import SwiftUI

/// Right panel: edit all locale values for a single key.
///
/// The source language row is shown first with a "base" badge.
/// Copy-from-source uses the source language value, not the first
/// alphabetically.
struct KeyDetailView: View {
    let key: KeyRow
    let fileName: String
    let sourceLanguage: String
    let displayedLocales: [String]
    let onSetValue: (String, String, Bool) -> Void  // (lang, value, isCopy)

    @State private var showCopied = false
    @State private var copiedLang: String?

    /// Source language value, used for copy-from-source.
    private var sourceValue: String? {
        if let val = key.translations[sourceLanguage], let v = val, !v.isEmpty {
            return v
        }
        return nil
    }

    /// Other locales to display, excluding the source language.
    private var otherLocales: [String] {
        displayedLocales.filter { $0 != sourceLanguage }
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                header
                Divider()

                // Source language row first
                if key.translations[sourceLanguage] != nil {
                    localeRow(sourceLanguage, isBase: true)
                    Divider().opacity(0.5)
                }

                // Other locales
                ForEach(otherLocales, id: \.self) { lang in
                    localeRow(lang, isBase: false)
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
    private func localeRow(_ lang: String, isBase: Bool) -> some View {
        let currentValue = key.translations[lang].flatMap { $0 }
        let isMissing = key.missingLocales.contains(lang)

        HStack(alignment: .top, spacing: 12) {
            // Locale badge
            HStack(spacing: 4) {
                Text(lang)
                    .font(.system(.body, design: .monospaced))
                if isBase {
                    Text("base")
                        .font(.system(size: 9, design: .monospaced))
                        .foregroundStyle(LocusTheme.violet)
                        .padding(.horizontal, 4)
                        .padding(.vertical, 1)
                        .background(LocusTheme.violet.opacity(0.15))
                        .clipShape(RoundedRectangle(cornerRadius: 3))
                }
            }
            .frame(width: 80, alignment: .leading)
            .foregroundStyle(isMissing ? .orange : .secondary)

            // Editable value
            TextField(
                "Missing…",
                text: Binding(
                    get: { currentValue ?? "" },
                    set: { newVal in onSetValue(lang, newVal, false) }
                )
            )
            .textFieldStyle(.roundedBorder)

            // Copy feedback (briefly visible after copy)
            if copiedLang == lang {
                Image(systemName: "checkmark")
                    .foregroundStyle(LocusTheme.violet)
                    .transition(.opacity)
            }

            // Copy-from-source button (always visible on non-base rows
            // when a source value exists, even after filling).
            if !isBase, let refVal = sourceValue {
                Button {
                    #if canImport(AppKit)
                    NSPasteboard.general.clearContents()
                    NSPasteboard.general.setString(refVal, forType: .string)
                    #endif
                    onSetValue(lang, refVal, true)
                    withAnimation { copiedLang = lang }
                    DispatchQueue.main.asyncAfter(deadline: .now() + 1.2) {
                        withAnimation { copiedLang = nil }
                    }
                } label: {
                    Image(systemName: "doc.on.clipboard")
                        .help("Copy from \(sourceLanguage)")
                }
                .buttonStyle(.borderless)
            }
        }
        .padding(.vertical, 2)
    }
}
