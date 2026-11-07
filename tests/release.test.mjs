import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import test from 'node:test';
import { syncVersions } from '../tools/sync-version.mjs';
import {
  applyVersion,
  buildNumber,
  numberOf,
  releaseTag,
  releaseTitle,
  versionOf,
} from '../tools/build-number.mjs';
import { previousRelease, releaseNotes } from '../tools/release-notes.mjs';

const root = path.resolve(import.meta.dirname, '..');
const metadata = ['appinfo.json', 'Cargo.toml', 'Cargo.lock', 'readme.md'];

async function fixture(t, version = '2.0.4') {
  const scratch = path.join(root, '.build');
  await fs.mkdir(scratch, { recursive: true });
  const directory = await fs.mkdtemp(path.join(scratch, 'release-test-'));
  t.after(() => fs.rm(directory, { recursive: true, force: true }));
  const project = JSON.parse(await fs.readFile(path.join(root, 'package.json'), 'utf8'));
  project.version = '1.2.3';
  await fs.writeFile(path.join(directory, 'package.json'), JSON.stringify(project));
  await Promise.all(
    metadata.map((filename) =>
      fs.copyFile(path.join(root, filename), path.join(directory, filename)),
    ),
  );
  await syncVersions(directory);
  project.version = version;
  await fs.writeFile(path.join(directory, 'package.json'), JSON.stringify(project));
  return directory;
}

const readMetadata = (directory) =>
  Promise.all(metadata.map((filename) => fs.readFile(path.join(directory, filename), 'utf8')));

test('version sync changes only release versions and is idempotent', async (t) => {
  const directory = await fixture(t, '3.1.0');
  const before = await readMetadata(directory);
  await assert.rejects(syncVersions(directory, { check: true }), /out of sync/);
  assert.deepEqual(await readMetadata(directory), before, 'check mode must not write files');

  assert.deepEqual((await syncVersions(directory)).changed, metadata);
  const after = await readMetadata(directory);
  const previous = JSON.parse(before[0]).version;
  assert.equal(JSON.parse(after[0]).version, '3.1.0');
  assert.equal(after[0], before[0].replace(`"version": "${previous}"`, '"version": "3.1.0"'));
  assert.equal(after[1], before[1].replace(`version = "${previous}"`, 'version = "3.1.0"'));
  assert.equal(
    after[2],
    before[2].replace(`name = "luma"\nversion = "${previous}"`, 'name = "luma"\nversion = "3.1.0"'),
    'dependency versions and helper crates must be preserved',
  );
  assert.equal(
    after[3],
    before[3].replaceAll(
      `com.eluz1ve.lumaplus_${previous}_arm.ipk`,
      'com.eluz1ve.lumaplus_3.1.0_arm.ipk',
    ),
  );
  assert.deepEqual((await syncVersions(directory, { check: true })).changed, []);
  assert.deepEqual((await syncVersions(directory)).changed, []);
});

test('invalid release versions and missing packages fail before writing metadata', async (t) => {
  for (const version of ['3.0.0-beta.1', '3.0', '03.0.0']) {
    const directory = await fixture(t, version);
    const before = await readMetadata(directory);
    await assert.rejects(syncVersions(directory), /numeric major.minor.patch/);
    assert.deepEqual(await readMetadata(directory), before);
  }
  const directory = await fixture(t);
  const lockfile = path.join(directory, 'Cargo.lock');
  await fs.writeFile(
    lockfile,
    (await fs.readFile(lockfile, 'utf8')).replace('name = "luma"', 'name = "other"'),
  );
  const before = await readMetadata(directory);
  await assert.rejects(syncVersions(directory), /one luma package in Cargo.lock/);
  assert.deepEqual(await readMetadata(directory), before);
});

test('a build is numbered by its minute in Chisinau and versioned by the same digits', () => {
  // 13:30 UTC is 15:30 in Chisinau in winter (EET) and 16:30 in summer (EEST).
  assert.equal(buildNumber(new Date('2026-11-08T13:30:00Z')), '2611081530');
  assert.equal(buildNumber(new Date('2026-07-01T13:30:00Z')), '2607011630');
  assert.equal(buildNumber(new Date('2026-12-31T22:05:00Z')), '2701010005');
  assert.equal(versionOf('2611081530'), '26.1108.1530');
  // No leading zeros, as the updater and Cargo require.
  assert.equal(versionOf('2701010005'), '27.101.5');
  assert.equal(versionOf('2701010000'), '27.101.0');
  for (const number of ['261108153', '2613081530', '2611082460', 'build']) {
    assert.throws(() => versionOf(number), /Invalid build number/);
  }
});

test('a build version names its release; versions before builds keep theirs', () => {
  for (const number of ['2611081530', '2701010005', '2701010000', '2612312359']) {
    assert.equal(numberOf(versionOf(number)), number);
  }
  assert.equal(releaseTag('26.1108.1530'), 'build-2611081530');
  assert.equal(releaseTitle('26.1108.1530'), 'Luma+ build #2611081530');
  assert.equal(numberOf('1.0.0'), null);
  assert.equal(releaseTag('1.0.0'), 'v1.0.0');
  assert.equal(releaseTitle('1.0.0'), 'Luma+ 1.0.0');
});

test('later builds are newer versions, and every build is newer than 1.0.0', () => {
  const compare = (a, b) => {
    const [x, y] = [a, b].map((version) => version.split('.').map(Number));
    return x[0] - y[0] || x[1] - y[1] || x[2] - y[2];
  };
  const builds = ['2611081530', '2611081531', '2611090005', '2612010000', '2701010000'];
  const versions = builds.map(versionOf);
  for (let index = 1; index < versions.length; index++) {
    assert.ok(compare(versions[index], versions[index - 1]) > 0, versions[index]);
  }
  for (const version of versions) assert.ok(compare(version, '1.0.0') > 0);
});

test('applying a build version writes it into package.json and every synced file', async (t) => {
  const directory = await fixture(t, '1.2.3');
  await syncVersions(directory);
  await applyVersion(directory, '26.1108.1530');
  assert.equal((await syncVersions(directory, { check: true })).version, '26.1108.1530');
  const [appinfo, , , readme] = await readMetadata(directory);
  assert.equal(JSON.parse(appinfo).version, '26.1108.1530');
  assert.match(readme, /com\.eluz1ve\.lumaplus_26\.1108\.1530_arm\.ipk/);
});

const commits = {
  'build-2611081530': 'b'.repeat(40),
  'build-2611091200': 'c'.repeat(40),
};
const release = (tag_name, published_at, extra = {}) => ({
  tag_name,
  name: tag_name,
  published_at,
  draft: false,
  target_commitish: 'main',
  ...extra,
});

test('the release before a build is the newest published one it descends from', () => {
  const releases = [
    release('v1.0.0', '2026-11-04T04:50:20Z', {
      name: 'Luma+ 1.0.0',
      target_commitish: 'a'.repeat(40),
    }),
    release('build-2611081530', '2026-11-08T13:50:00Z'),
    release('build-2611091200', '2026-11-09T10:20:00Z'),
    release('build-2611091300', '2026-11-09T11:20:00Z', { draft: true }),
  ];
  const resolve = (tag) => commits[tag] ?? null;
  const found = previousRelease(releases, resolve, () => true);
  assert.equal(found.release.tag_name, 'build-2611091200');
  assert.equal(found.commit, 'c'.repeat(40));
  // A build on another line of history is passed over.
  const other = previousRelease(releases, resolve, (commit) => commit !== 'c'.repeat(40));
  assert.equal(other.release.tag_name, 'build-2611081530');
  // Before builds, a release counts from the commit it was published at, never its tag.
  const first = previousRelease(releases.slice(0, 1), resolve, () => true);
  assert.equal(first.commit, 'a'.repeat(40));
  assert.equal(
    previousRelease([release('v0.9.0', '2026-10-01T00:00:00Z')], resolve, () => true),
    null,
  );
});

test('release notes list the commits since the release before, linked to their comparison', () => {
  const notes = releaseNotes({
    subjects: [
      'Lay the page under an overlay down as one picture',
      'Launch wallpaper fade at 60 Hz',
    ],
    repository: 'eluz1ve/lg-luma-plus',
    base: 'e7b6ae64f92157d4928eb5ab6b8590bafdc51888',
    head: 'd6fc86c0000000000000000000000000000000000',
    previous: 'Luma+ 1.0.0',
  });
  assert.equal(
    notes,
    [
      '### Changes',
      '',
      '- Lay the page under an overlay down as one picture',
      '- Launch wallpaper fade at 60 Hz',
      '',
      'Since Luma+ 1.0.0: [e7b6ae6...d6fc86c](https://github.com/eluz1ve/lg-luma-plus/compare/e7b6ae6...d6fc86c)',
      '',
    ].join('\n'),
  );
  assert.match(
    releaseNotes({ subjects: [], repository: 'a/b', base: null, head: 'f' }),
    /- Rebuilt/,
  );
});
