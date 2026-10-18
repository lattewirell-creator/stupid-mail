import Foundation
import CryptoKit

// Verify against the public key shipped in the app, without reading Keychain.
let args = CommandLine.arguments
let key = try Curve25519.Signing.PublicKey(rawRepresentation: Data(base64Encoded: args[1])!)
let signature = Data(base64Encoded: args[2])!
let archive = try Data(contentsOf: URL(fileURLWithPath: args[3]))
precondition(key.isValidSignature(signature, for: archive), "Release signature invalid")
var changed = archive
changed[changed.startIndex] ^= 1
precondition(!key.isValidSignature(signature, for: changed), "Tampered release was accepted")
let wrongKey = Curve25519.Signing.PrivateKey().publicKey
precondition(!wrongKey.isValidSignature(signature, for: archive), "Wrong key accepted")
print("Update signature: genuine archive accepted; modified archive and wrong key rejected")
