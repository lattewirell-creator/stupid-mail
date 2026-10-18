# App updates

Install version 2.5 once, replacing your older copy in Applications. Earlier versions do not contain an updater and cannot upgrade themselves. Keep the app in a writable location, rather than running inside a download preview or read-only disk image.

The app uses Sparkle 2.10.0. Automatic checks and background downloads are enabled initially. It checks on its launch schedule and hourly while running, subject to connectivity and GitHub caching. This is not instant push delivery. Use **The Stupid Mail → Check for Updates…** to check now, or toggle **Automatic Updates** in that menu. Preferences persist across launches.

Downloaded updates install on quit, or through Sparkle's update window. Mail's existing quit guard waits for sending/Buddy requests and saves drafts before allowing termination. Settings, Keychain accounts, and cached data live outside the app bundle. Updating replaces the application, not those stores. The updater does not run while Mail is closed.

## Publish future updates

A source commit alone is not an installable update. Every release must include the Mac archive and signed appcast:

1. Increase `version` and the monotonically increasing `build` in `updates/config.json`. Never change the public key without a planned key migration.
2. Run `python3 scripts/test.py` and `python3 scripts/build-mac.py`. The build fetches a pinned official Sparkle distribution, checks its SHA-256, and embeds it with its license and original signatures.
3. Run `python3 scripts/sign-release.py`, then `python3 scripts/test-update.py`. Signing uses the `stupid-mail` account in the local login Keychain. Keep that private key backed up securely; never commit or upload it.
4. Commit and push source/config changes **without the new appcast yet**. Create the GitHub release for that version and upload its ZIP. GitHub normalizes the filename to `The.Stupid.Mail.Mac.zip`; the feed expects that exact asset name. Verify the release and asset are public and fully uploaded.
5. Commit and push `updates/appcast.xml` last. The feed points at the versioned, immutable release asset. Never replace an already published ZIP without issuing a new build and signature.

Both the archive and feed are signed with Ed25519; the public key is embedded in the app. Signature checks happen before extraction. The key remains on the publishing Mac, not GitHub. Sparkle system profiling is disabled; update requests still reveal normal connection metadata to GitHub.

The app remains ad-hoc signed, not Apple-notarized. Automated checks verify the archive's public-key signature (including tampered/wrong-key rejection), feed consistency, framework embedding, and code signatures. An actual quit/install/relaunch update still needs testing with a later release on a running installation; these checks do not claim end-to-end installation verification.
