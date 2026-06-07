const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const iconsDir = path.join(__dirname, '..', 'src-tauri', 'icons');

const sizes = [
  { name: '32x32.png', size: 32 },
  { name: '128x128.png', size: 128 },
  { name: '128x128@2x.png', size: 256 },
  { name: 'icon.icns', size: 512 },
  { name: 'icon.ico', size: 256 },
  { name: 'icon.png', size: 512 },
];

const svgContent = `
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  <rect width="512" height="512" rx="64" fill="#4F46E5"/>
  <path d="M64 160L256 320L448 160V384C448 401.674 433.674 416 416 416H96C78.326 416 64 401.674 64 384V160Z" fill="white" fill-opacity="0.95"/>
  <path d="M64 160L256 288L448 160L448 128C448 110.326 433.674 96 416 96H96C78.326 96 64 110.326 64 128L64 160Z" fill="#818CF8"/>
  <path d="M256 288L64 160L64 128C64 110.326 78.326 96 96 96H416C433.674 96 448 110.326 448 128L448 160L256 288Z" fill="#6366F1" stroke="#4F46E5" stroke-width="2"/>
</svg>
`;

fs.writeFileSync(path.join(iconsDir, 'icon.svg'), svgContent);

try {
  const sharp = require('sharp');
  console.log('Using sharp to generate icons...');
  
  (async () => {
    for (const { name, size } of sizes) {
      if (name.endsWith('.png')) {
        await sharp(Buffer.from(svgContent))
          .resize(size, size)
          .png()
          .toFile(path.join(iconsDir, name));
        console.log(`Generated ${name}`);
      }
    }
    
    await sharp(Buffer.from(svgContent))
      .resize(512, 512)
      .png()
      .toFile(path.join(iconsDir, 'icon.png'));
    console.log('Generated icon.png');
    
    console.log('\nNote: For .ico and .icns files, please use tauri icon command or online tools.');
    console.log('Run: npm run tauri icon src-tauri/icons/icon.png');
  })();
} catch (e) {
  console.log('Sharp not available, using fallback method...');
  console.log('\nPlease install sharp: npm install sharp -D');
  console.log('Or use Tauri\'s built-in icon generator:');
  console.log('npm run tauri icon src-tauri/icons/icon.svg');
}
