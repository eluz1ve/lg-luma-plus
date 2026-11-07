// Luma+ builds are numbered by when they are made, in the maintainer's time zone: build
// 2611081530 was made on 8 November 2026 at 15:30. Its app version, 26.1108.1530, is the same
// digits split in three, which webOS and the updater compare as numbers: a later build is always
// a newer version, and newer than every 1.x release before builds.
//
// `node tools/build-number.mjs` prints the build's number, version, tag and title (and writes them
// to GITHUB_OUTPUT in Actions). `--apply` writes the version into package.json and synchronizes
// every versioned file from it. LUMA_BUILD_NUMBER reuses a number, so one CI run's jobs agree.
import fs from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { syncVersions } from './sync-version.mjs';

export const TIME_ZONE = 'Europe/Chisinau';

/** The number of a build made at `date`: its year, month, day, hour and minute, YYMMDDHHmm. */
export function buildNumber(date = new Date(), timeZone = TIME_ZONE) {
  const parts = Object.fromEntries(
    new Intl.DateTimeFormat('en-GB', {
      timeZone,
      year: '2-digit',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      hourCycle: 'h23',
    })
      .formatToParts(date)
      .map((part) => [part.type, part.value]),
  );
  return `${parts.year}${parts.month}${parts.day}${parts.hour}${parts.minute}`;
}

function parts(number) {
  const match = /^(\d{2})(\d{2})(\d{2})(\d{2})(\d{2})$/.exec(number);
  if (!match) return null;
  const [year, month, day, hour, minute] = match.slice(1).map(Number);
  if (month < 1 || month > 12 || day < 1 || day > 31 || hour > 23 || minute > 59) return null;
  return { year, month, day, hour, minute };
}

/** A build's app version: 2611081530 → 26.1108.1530. */
export function versionOf(number) {
  const time = parts(number);
  if (!time) throw new Error(`Invalid build number: ${number}`);
  return [time.year, time.month * 100 + time.day, time.hour * 100 + time.minute].join('.');
}

/** The build an app version was made as, or null for a version from before builds. */
export function numberOf(version) {
  const match = /^(0|[1-9]\d?)\.([1-9]\d{2,3})\.(0|[1-9]\d{0,3})$/.exec(version);
  if (!match) return null;
  const number = [
    match[1].padStart(2, '0'),
    match[2].padStart(4, '0'),
    match[3].padStart(4, '0'),
  ].join('');
  return parts(number) ? number : null;
}

/** The release tag of an app version: build-2611081530, or v1.0.0 from before builds. */
export function releaseTag(version) {
  const number = numberOf(version);
  return number ? `build-${number}` : `v${version}`;
}

/** The release title of an app version. */
export function releaseTitle(version) {
  const number = numberOf(version);
  return number ? `Luma+ build #${number}` : `Luma+ ${version}`;
}

/** Writes `version` into package.json, then every file synchronized from it. */
export async function applyVersion(directory, version) {
  const file = path.join(directory, 'package.json');
  const source = await fs.readFile(file, 'utf8');
  // The first "version" key is the package's own; the text is kept as formatted.
  const updated = source.replace(/("version"\s*:\s*")[^"\r\n]+(")/, `$1${version}$2`);
  if (JSON.parse(updated).version !== version) {
    throw new Error('Expected the package version first in package.json.');
  }
  await fs.writeFile(file, updated);
  return syncVersions(directory);
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) {
  const root = path.resolve(import.meta.dirname, '..');
  const number = process.env.LUMA_BUILD_NUMBER || buildNumber();
  const version = versionOf(number);
  const build = { number, version, tag: releaseTag(version), title: releaseTitle(version) };
  if (process.argv.includes('--apply')) await applyVersion(root, version);
  if (process.env.GITHUB_OUTPUT) {
    await fs.appendFile(
      process.env.GITHUB_OUTPUT,
      Object.entries(build)
        .map(([key, value]) => `${key}=${value}\n`)
        .join(''),
    );
  }
  console.log(JSON.stringify(build));
}
