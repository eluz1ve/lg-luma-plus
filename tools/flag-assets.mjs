import fs from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import sharp from 'sharp';

const root = path.resolve(import.meta.dirname, '..');
// Twice what the CS2 match page draws (36×27), so a 4K interface stays sharp.
const WIDTH = 72;
const HEIGHT = 54;

export async function generateFlagAssets() {
  const source = path.join(root, 'assets/flags');
  const out = path.join(root, '.build/assets/flags');
  await fs.rm(out, { recursive: true, force: true });
  await fs.mkdir(out, { recursive: true });
  for (const file of await fs.readdir(source)) {
    if (!file.endsWith('.svg')) continue;
    await sharp(path.join(source, file))
      .resize(WIDTH, HEIGHT, { fit: 'fill' })
      .png({ palette: true, compressionLevel: 9 })
      .toFile(path.join(out, `${path.parse(file).name}.png`));
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  await generateFlagAssets();
}
