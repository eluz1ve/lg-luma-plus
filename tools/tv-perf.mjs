// Measures frame times on the TV: for each variant, writes the app's perf switch file (a scripted
// walk through Home, the menu, Today and Settings, plus the variant's launch options), relaunches
// Luma+, saves perf.log to dist/perf/<variant>.log, and prints one summary row per variant.
// The walk never presses OK (`today` and `side-menu` open those without moving the focus), so no
// app or poster launches. Afterwards the switch file is left as
// it was found.
//
//   node tools/tv-perf.mjs [--device lgtv] [--seconds 110] [variant ...]
//
// Variants: base (as installed), msaa0, msaa2, nopartial, empty (draws nothing: the compositor
// and swap alone), or any `name=line;line` for custom switch-file lines.
import fs from 'node:fs/promises';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const root = path.resolve(import.meta.dirname, '..');
const manifest = JSON.parse(await fs.readFile(path.join(root, 'appinfo.json'), 'utf8'));
const args = process.argv.slice(2);
const option = (name, fallback) => {
  const index = args.indexOf(name);
  if (index < 0) return fallback;
  const [value] = args.splice(index, 2).slice(1);
  return value;
};
const device = option('--device', 'lgtv');
const seconds = Number(option('--seconds', '110'));
if (!/^[\w-]+$/.test(device)) throw new Error('Pass an ares device profile: --device lgtv');
if (!(seconds > 5)) throw new Error('--seconds must be more than 5');

const known = {
  base: [],
  msaa0: ['msaa=0'],
  msaa2: ['msaa=2'],
  nopartial: ['partial=0'],
  empty: ['skip=all'],
};
const variants = (args.length ? args : ['base', 'msaa0', 'nopartial', 'empty']).map((name) => {
  const custom = name.match(/^([\w-]+)=(.+)$/);
  if (custom) return { name: custom[1], lines: custom[2].split(';') };
  if (!known[name]) throw new Error(`Unknown variant ${name}: ${Object.keys(known).join(', ')}`);
  return { name, lines: known[name] };
});

const pause = (count) => Array(count).fill('wait');
const press = (key, count, settle) =>
  Array.from({ length: count }, () => [key, ...pause(settle)]).flat();
// Each step takes three frames: ~50 ms while something moves, ~300 ms on a still screen, so
// the whole walk takes about 90 s.
const walk = [
  ...pause(20),
  ...press('right', 4, 6),
  ...press('left', 4, 6),
  'side-menu',
  ...pause(14),
  'back',
  ...pause(14),
  ...press('up', 1, 8),
  ...press('right', 3, 6),
  ...press('left', 3, 6),
  'today', // opens the Today card wherever the focus is
  ...pause(16),
  ...press('right', 2, 10),
  'back',
  ...pause(16),
  'settings',
  ...pause(16),
  ...press('down', 4, 6),
  'home',
  ...pause(20),
];

const cli = (tool, toolArgs, { allowFailure = false, capture = false } = {}) => {
  const result = spawnSync(
    process.execPath,
    [
      path.join(root, 'node_modules/@webos-tools/cli/bin', `${tool}.js`),
      '--device',
      device,
      ...toolArgs,
    ],
    { cwd: root, encoding: 'utf8', stdio: capture ? 'pipe' : 'inherit', timeout: 120000 },
  );
  if (result.error) throw result.error;
  if (result.status !== 0 && !allowFailure)
    throw new Error(`${tool} failed: ${(result.stderr || '').trim()}`);
  return result.stdout || '';
};
// Only the app's own perf files are touched; nothing else there is read. ares-novacom prints
// "[Info] Set target device : …" ahead of the command's own output.
const shell = (command) =>
  cli('ares-novacom', ['--run', command], { capture: true }).replace(/^\[Info\] .*\n/, '');
const quote = (text) => `'${text.replaceAll("'", `'\\''`)}'`;

const directory = shell(
  [
    `/media/developer/${manifest.id}`,
    `/media/developer/temp/${manifest.id}`,
    `/media/internal/.${manifest.id}`,
  ]
    .map((candidate) => `[ -e ${candidate}/preferences.json ] && echo ${candidate} && exit 0`)
    .join('; ') + '; exit 0',
)
  .trim()
  .split('\n')[0];
if (!directory)
  throw new Error(`No ${manifest.id} preferences on the TV; launch Luma+ once first.`);
const switchFile = `${directory}/perf`;
const existed = shell(`[ -e ${switchFile} ] && echo yes; echo`).trim() === 'yes';
// The switch file holds only perf options.
const saved = existed ? shell(`cat ${switchFile}`) : '';

const out = path.join(root, 'dist/perf');
await fs.mkdir(out, { recursive: true });
const rows = [];
try {
  for (const variant of variants) {
    console.log(`\n${variant.name}: ${variant.lines.join(', ') || 'as installed'}`);
    cli('ares-launch', ['--close', manifest.id], { allowFailure: true });
    const lines = [`script=${walk.join(',')}`, ...variant.lines];
    shell(
      `printf '%s\\n' ${lines.map(quote).join(' ')} > ${switchFile} && : > ${directory}/perf.log`,
    );
    cli('ares-launch', [manifest.id]);
    await new Promise((resolve) => setTimeout(resolve, seconds * 1000));
    const log = shell(`cat ${directory}/perf.log`);
    await fs.writeFile(path.join(out, `${variant.name}.log`), log);
    rows.push(summary(variant.name, log));
  }
} finally {
  if (existed) shell(`printf '%s' ${quote(saved)} > ${switchFile}`);
  else shell(`rm -f ${switchFile}`);
  cli('ares-launch', ['--close', manifest.id], { allowFailure: true });
  cli('ares-launch', [manifest.id], { allowFailure: true });
}
console.log(`\nLogs in ${path.relative(root, out)}/\n`);
console.log('variant      frames  gap ms  late  gpu ms  start');
for (const row of rows) console.log(row);

/** Animated seconds only (10+ frames): frame-weighted mean gap and GPU time, late frames. */
function summary(name, log) {
  let frames = 0;
  let gap = 0;
  let late = 0;
  let gpu = 0;
  let gpuFrames = 0;
  const start = log.match(/start (.*)/)?.[1] ?? '';
  for (const line of log.split('\n')) {
    const n = Number(line.match(/ n=(\d+)/)?.[1] ?? 0);
    if (n < 10) continue;
    frames += n;
    gap += n * Number(line.match(/ gap ([\d.]+)/)?.[1] ?? 0);
    late += Number(line.match(/ late (\d+)/)?.[1] ?? 0);
    const g = line.match(/ gpu ([\d.]+)/);
    if (g) {
      gpu += n * Number(g[1]);
      gpuFrames += n;
    }
  }
  const mean = (sum, count) => (count ? (sum / count).toFixed(1) : '-');
  return [
    name.padEnd(12),
    String(frames).padStart(6),
    mean(gap, frames).padStart(7),
    String(late).padStart(5),
    mean(gpu, gpuFrames).padStart(7),
    ` ${start}`,
  ].join(' ');
}
