import SwiftUI

/// Popover for filtering which locales to display.
/// Stays open while toggling checkboxes.
struct LocaleFilterPopover: View {
    let allLocales: [String]
    @Binding var visibleLocales: Set<String>

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // Header
            HStack {
                Text("Filter Locales")
                    .font(.headline)
                Spacer()
                Button("All") { visibleLocales = [] }
                    .buttonStyle(.borderless)
                    .font(.caption)
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 8)

            Divider()

            // Locale list with toggles — no scroll indicator
            ScrollView(showsIndicators: false) {
                VStack(spacing: 0) {
                    ForEach(allLocales, id: \.self) { lang in
                        Toggle(isOn: Binding(
                            get: { visibleLocales.contains(lang) },
                            set: { isOn in
                                if isOn {
                                    visibleLocales.insert(lang)
                                } else {
                                    visibleLocales.remove(lang)
                                }
                            }
                        )) {
                            Text(lang)
                                .font(.system(.body, design: .monospaced))
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(.horizontal, 12)
                        .padding(.vertical, 4)
                    }
                }
            }
        }
        .frame(width: 200, height: min(CGFloat(allLocales.count * 32 + 60), 400))
    }
}
