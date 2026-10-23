from pathlib import Path
import subprocess,shutil,struct,plistlib,json
root=Path(__file__).resolve().parents[1]
def run(*args):subprocess.run([str(x) for x in args],check=True)
config=json.loads((root/'updates/config.json').read_text())
run('python3',root/'scripts/prepare-sparkle.py')
sparkle=root/'.build/sparkle'
run('python3',root/'scripts/bundle-buddy.py')
run('node','--check',root/'dist/app.js');run('node','--check',root/'dist/buddy-ui.js');run('node','--check',root/'dist/buddy3d.bundle.js')
release=root/('release-'+config['version']);release.mkdir(exist_ok=True)
app=release/'app.noindex'/'The Stupid Mail.app';   # ".noindex": Spotlight and Launchpad don't list build copies as extra apps
mac=app/'Contents/MacOS';resources=app/'Contents/Resources';mac.mkdir(parents=True,exist_ok=True);resources.mkdir(parents=True,exist_ok=True)
# One app for Apple chip and Intel Macs: build each, then join them.
sources=[root/'macos'/x for x in ['App.swift','MailCore.swift','IMAPConnection.swift','NativeBridge.swift','BuddyAI.swift','MailCache.swift','OutlookMail.swift','AccountSetup.swift','Workspace.swift','WorkspaceCore.swift','CalendarCore.swift','CalendarBridge.swift']]
slices=[]
for arch in ['arm64','x86_64']:
 out=root/'.build'/f'The Stupid Mail-{arch}';out.parent.mkdir(exist_ok=True)
 run('swiftc',*sources,'-O','-o',out,'-F',sparkle,'-framework','Sparkle','-Xlinker','-rpath','-Xlinker','@executable_path/../Frameworks','-framework','AppKit','-framework','WebKit','-framework','Security','-framework','Network','-framework','EventKit','-target',f'{arch}-apple-macos13.0','-module-cache-path',f'/private/tmp/stupid-mail-swift-cache-{arch}')
 slices.append(out)
run('lipo','-create',*slices,'-output',mac/'The Stupid Mail')
shutil.copy2(root/'macos/AppIcon-1024.png',root/'macos/mail-icon-1024.png')   # drawn from macos/AppIcon.svg
shutil.copy2(root/'macos/mail-icon-1024.png',root/'dist/mail-icon.png')
chunks=[]
for name,n in [('icp4',16),('icp5',32),('icp6',64),('ic07',128),('ic08',256),('ic09',512),('ic10',1024)]:
 p=root/'macos'/f'icon-{n}.png'
 subprocess.run(['sips','-z',str(n),str(n),str(root/'macos/mail-icon-1024.png'),'--out',str(p)],check=True,stdout=subprocess.DEVNULL)
 data=p.read_bytes();chunks.append(name.encode()+struct.pack('>I',len(data)+8)+data)
data=b''.join(chunks);(resources/'AppIcon.icns').write_bytes(b'icns'+struct.pack('>I',len(data)+8)+data)
shutil.copytree(root/'dist',resources/'Web',dirs_exist_ok=True)
info={'CFBundleName':'The Stupid Mail','CFBundleDisplayName':'The Stupid Mail','CFBundleIdentifier':'local.emil.thestupidmail','CFBundleVersion':str(config['build']),'CFBundleShortVersionString':config['version'],'CFBundleExecutable':'The Stupid Mail','CFBundlePackageType':'APPL','CFBundleIconFile':'AppIcon','LSMinimumSystemVersion':'13.0','NSCalendarsUsageDescription':'Show your upcoming events and save meeting suggestions you confirm.','NSCalendarsFullAccessUsageDescription':'Show your upcoming events and save meeting suggestions you confirm.','NSHighResolutionCapable':True,'NSPrincipalClass':'NSApplication','NSAppTransportSecurity':{'NSAllowsLocalNetworking':True}}
info.update({'SUFeedURL':config['feed_url'],'SUPublicEDKey':config['public_key'],'SUEnableAutomaticChecks':True,'SUAutomaticallyUpdate':True,'SUScheduledCheckInterval':3600,'SUVerifyUpdateBeforeExtraction':True,'SURequireSignedFeed':True,'SUSendProfileInfo':False})
frameworks=app/'Contents/Frameworks';frameworks.mkdir(exist_ok=True)
if (frameworks/'Sparkle.framework').exists():shutil.rmtree(frameworks/'Sparkle.framework')
shutil.copytree(sparkle/'Sparkle.framework',frameworks/'Sparkle.framework',symlinks=True)
shutil.copy2(sparkle/'LICENSE',resources/'Sparkle-LICENSE.txt')
(app/'Contents/Info.plist').write_bytes(plistlib.dumps(info))
run('codesign','--force','--sign','-',app);run('codesign','--verify','--deep','--strict',app)
run('ditto','-c','-k','--sequesterRsrc','--keepParent',app,release/'The Stupid Mail — Mac.zip')
print('Built and verified:',app)
