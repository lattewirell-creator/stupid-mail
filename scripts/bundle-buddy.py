from pathlib import Path
import re
root=Path(__file__).resolve().parents[1]/'dist'
source=(root/'vendor/three.module.min.js').read_text()
match=re.search(r'export\s*\{([^}]+)\}\s*;?\s*$',source)
assert match,'Unexpected Three.js export structure'
exports=[]
for item in match.group(1).split(','):
 parts=re.split(r'\s+as\s+',item.strip())
 exports.append(parts[-1]+':'+parts[0])
bundle="/* Bundled for reliable file-based macOS loading. See vendor/THREE-LICENSE. */\n(()=>{\nconst THREE=(()=>{\n"+source[:match.start()]+"\nreturn {"+','.join(exports)+"};})();\n"
for name in ['RoundedBoxGeometry','RoomEnvironment']:
 source=(root/f'vendor/{name}.js').read_text()
 source=re.sub(r"import\s*\{([^}]+)\}\s*from\s*'[^']+';",r'const {\1}=THREE;',source)
 source=re.sub(r'export\s*\{[^}]+\};','',source)
 bundle+=f'const {name}=(()=>{{\n'+source+f'\nreturn {name};}})();\n'
source=(root/'buddy3d.js').read_text()
source=re.sub(r'^import .*;\n','',source,flags=re.M).replace('export function createBuddy','function createBuddy')
bundle+=source+'\n})();\n'
(root/'buddy3d.bundle.js').write_text(bundle)
