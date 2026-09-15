import Foundation

// UniFFI generates KeyRow without Identifiable conformance.
// Table requires it, so we add it here.
extension KeyRow: Identifiable {
    public var id: String { key }
}
