"""Check a signed release's feed, archive signature, and embedded framework."""
from pathlib import Path
import json
import plistlib
import subprocess
import tempfile
import xml.etree.ElementTree as ET
root = Path(__file__).resolve().parents[1]
config = json.loads((root/'updates/config.json').read_text())
release = root/('release-'+config['version'])
app = release/'The Stupid Mail.app'
archive = release/'The Stupid Mail — Mac.zip'
info = plistlib.loads((app/'Contents/Info.plist').read_bytes())
feed = ET.parse(root/'updates/appcast.xml')
ns = {'s': 'http://www.andymatuschak.org/xml-namespaces/sparkle'}
item = feed.find('channel/item')
enclosure = item.find('enclosure')
assert item.find('s:version', ns).text == info['CFBundleVersion'] == str(config['build'])
assert item.find('s:shortVersionString', ns).text == info['CFBundleShortVersionString'] == config['version']
assert int(enclosure.attrib['length']) == archive.stat().st_size
assert enclosure.attrib['url'] == f'https://github.com/lattewirell-creator/stupid-mail/releases/download/v{config["version"]}/The.Stupid.Mail.Mac.zip'
assert info['SUPublicEDKey'] == config['public_key']
assert info['SUFeedURL'] == config['feed_url']
assert info['SURequireSignedFeed'] and info['SUVerifyUpdateBeforeExtraction']
assert info['SUEnableAutomaticChecks'] and info['SUAutomaticallyUpdate']
assert info['SUScheduledCheckInterval'] == 3600
framework = app/'Contents/Frameworks/Sparkle.framework'
assert (framework/'Sparkle').is_file() and (framework/'Versions/Current').is_symlink()
subprocess.run(['codesign','--verify','--deep','--strict',str(app)],check=True)
with tempfile.TemporaryDirectory(prefix='stupid-mail-update-test-') as tmp:
    binary = Path(tmp)/'verify'
    subprocess.run(['swiftc',str(root/'tests/UpdateSignatureTests.swift'),'-o',str(binary),'-module-cache-path','/private/tmp/stupid-mail-test-module-cache'],check=True)
    subprocess.run([str(binary),config['public_key'],enclosure.attrib['{'+ns['s']+'}edSignature'],str(archive)],check=True)
print('Release feed, version, framework, automatic update configuration, and code signature checks passed')
