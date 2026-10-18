const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const context={};context.window=context;vm.runInNewContext(fs.readFileSync('dist/buddy-look.js','utf8'),context);
const B=context.BuddyLook,normal=B.normalize();
assert.equal(normal.shape,'hiker');assert.equal(normal.color,'cream');assert.equal(normal.glow,'cyan');assert.equal(normal.outfit,'clay');assert.equal(normal.eyes,'dot');assert.equal(normal.mouth,'smile');assert.ok(normal.backpack&&normal.scarf);
for(const [key,values]of Object.entries(B.choices))for(const value of values)assert.equal(B.normalize({[key]:value})[key],value);
for(const key of ['color','glow','outfit'])assert.equal(B.normalize({[key]:'#123abc'})[key],'#123abc');
assert.equal(B.normalize({image:'https://tracker.example/pic.png'}).image,'');assert.equal(B.normalize({image:'data:image/svg+xml;base64,AAAA'}).image,'');assert.equal(B.normalize({image:'data:image/jpeg;base64,AAAA'}).image,'data:image/jpeg;base64,AAAA');assert.equal(B.normalize({shape:'invalid',color:'bad'}).shape,'hiker');assert.equal(B.normalize({backpack:false,scarf:false}).backpack,false);
const html=fs.readFileSync('dist/index.html','utf8');for(const key of Object.keys(B.choices))assert.ok(html.includes('id="buddy-'+key+'"'));
console.log('Passed browser-default parity, every appearance option, custom colours, gear toggles, image validation, and control coverage.');
