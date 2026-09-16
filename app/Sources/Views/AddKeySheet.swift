import SwiftUI

/// Sheet for adding a new localization key with a base-locale value.
struct AddKeySheet: View {
    let files: [FileInfo]
    let selectedFile: String?
    let onAdd: (String, String, String, String) -> Void  // (file, key, lang, value)

    @State private var keyName = ""
    @State private var baseValue = ""
    @State private var targetFile: String = ""
    @State private var baseLang: String = "en"
    @Environment(\.dismiss) private var dismiss

    private var isValid: Bool {
        !keyName.trimmingCharacters(in: .whitespaces).isEmpty
        && !baseValue.trimmingCharacters(in: .whitespaces).isEmpty
        && !targetFile.isEmpty
    }

    var body: some View {
        VStack(spacing: 16) {
            Text("Add Key")
                .font(.headline)

            // File picker
            Picker("File", selection: $targetFile) {
                ForEach(files, id: \.name) { file in
                    Text(file.name).tag(file.name)
                }
            }
            .pickerStyle(.menu)

            // Base language picker (from the selected file's locales)
            if let file = files.first(where: { $0.name == targetFile }) {
                Picker("Base language", selection: $baseLang) {
                    ForEach(file.locales, id: \.self) { lang in
                        Text(lang).tag(lang)
                    }
                }
                .pickerStyle(.menu)
            }

            // Key name
            TextField("Key name (e.g. auth.login.button)", text: $keyName)
                .textFieldStyle(.roundedBorder)

            // Base value
            TextField("Base value (e.g. Sign In)", text: $baseValue)
                .textFieldStyle(.roundedBorder)

            HStack {
                Button("Cancel") { dismiss() }
                    .keyboardShortcut(.cancelAction)
                Spacer()
                Button("Add") {
                    onAdd(targetFile, keyName, baseLang, baseValue)
                }
                .keyboardShortcut(.defaultAction)
                .disabled(!isValid)
            }
        }
        .padding(20)
        .frame(width: 400)
        .onAppear {
            // Pre-select the currently open file and its source language.
            if let sf = selectedFile {
                targetFile = sf
            } else if let first = files.first {
                targetFile = first.name
            }
            if let file = files.first(where: { $0.name == targetFile }) {
                baseLang = file.sourceLanguage.isEmpty ? "en" : file.sourceLanguage
            }
        }
    }
}
