from pathlib import Path
import subprocess,shutil,struct,plistlib
root=Path(__file__).resolve().parents[1]
def run(*args):subprocess.run([str(x) for x in args],check=True)
run('python3',root/'scripts/bundle-buddy.py')
run('node','--check',root/'dist/app.js');run('node','--check',root/'dist/buddy-ui.js');run('node','--check',root/'dist/buddy3d.bundle.js')
release=root/'release-2.1';release.mkdir(exist_ok=True)
app=release/'The Stupid Mail.app';mac=app/'Contents/MacOS';resources=app/'Contents/Resources';mac.mkdir(parents=True,exist_ok=True);resources.mkdir(parents=True,exist_ok=True)
run('swiftc',*[root/'macos'/x for x in ['App.swift','MailCore.swift','IMAPConnection.swift','NativeBridge.swift','BuddyAI.swift','MailCache.swift']],'-o',mac/'The Stupid Mail','-framework','AppKit','-framework','WebKit','-framework','Security','-framework','Network','-target','arm64-apple-macos13.0','-module-cache-path','/private/tmp/stupid-mail-swift-cache')
run('swiftc',root/'macos/Icon.swift','-o','/private/tmp/stupid-mail-icon-maker','-framework','AppKit','-module-cache-path','/private/tmp/stupid-mail-swift-cache')
run('/private/tmp/stupid-mail-icon-maker',root/'macos/mail-icon-1024.png')
shutil.copy2(root/'macos/mail-icon-1024.png',root/'dist/mail-icon.png')
chunks=[]
for name,n in [('icp4',16),('icp5',32),('icp6',64),('ic07',128),('ic08',256),('ic09',512),('ic10',1024)]:
 p=root/'macos'/f'icon-{n}.png'
 subprocess.run(['sips','-z',str(n),str(n),str(root/'macos/mail-icon-1024.png'),'--out',str(p)],check=True,stdout=subprocess.DEVNULL)
 data=p.read_bytes();chunks.append(name.encode()+struct.pack('>I',len(data)+8)+data)
data=b''.join(chunks);(resources/'AppIcon.icns').write_bytes(b'icns'+struct.pack('>I',len(data)+8)+data)
shutil.copytree(root/'dist',resources/'Web',dirs_exist_ok=True)
info={'CFBundleName':'The Stupid Mail','CFBundleDisplayName':'The Stupid Mail','CFBundleIdentifier':'local.emil.thestupidmail','CFBundleVersion':'3','CFBundleShortVersionString':'2.1','CFBundleExecutable':'The Stupid Mail','CFBundlePackageType':'APPL','CFBundleIconFile':'AppIcon','LSMinimumSystemVersion':'13.0','NSHighResolutionCapable':True,'NSPrincipalClass':'NSApplication','NSAppTransportSecurity':{'NSAllowsLocalNetworking':True}}
(app/'Contents/Info.plist').write_bytes(plistlib.dumps(info))
run('codesign','--force','--sign','-',app);run('codesign','--verify','--deep','--strict',app)
run('ditto','-c','-k','--sequesterRsrc','--keepParent',app,release/'The Stupid Mail — Mac.zip')
print('Built and verified:',app)
