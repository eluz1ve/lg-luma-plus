import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import test from 'node:test';

const source = fs.readFileSync(new URL('../tv/self-update.sh', import.meta.url), 'utf8');
function run({
  install = true,
  home = false,
  reapply = true,
  recover = true,
  launch = true,
  checksum = true,
  method = 'executable',
  homeId = 'com.webos.app.home',
  pretty = false,
  stateMode = null,
  stateOwner = 0,
  fixtures = () => {},
} = {}) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'luma-installer-'));
  const app = `${dir}/app`;
  const state = `${dir}/update`;
  const homeState = `${dir}/home`;
  const bin = `${dir}/bin`;
  for (const d of [app, homeState, bin]) fs.mkdirSync(d);
  if (stateMode !== null) {
    fs.mkdirSync(state);
    fs.chmodSync(state, stateMode);
  }
  const write = (file, contents) => fs.writeFileSync(file, contents, { mode: 0o755 });
  const attempt = 'a'.repeat(32);
  const download = (name) => {
    fs.mkdirSync(`${dir}/${name}`);
    write(`${dir}/${name}/package.ipk`, 'test package');
    return `${dir}/${name}`;
  };
  download(`luma-update-${attempt}`);
  fixtures({ dir, download });
  // tools/package.mjs and the native helper pretty-print; the saved stock manifest has its own
  // "version" and "id", which must not be mistaken for Luma's version or the Home target.
  const json = (value) => (pretty ? `${JSON.stringify(value, null, 2)}\n` : JSON.stringify(value));
  write(
    `${app}/appinfo.json`,
    json({ id: 'com.eluz1ve.lumaplus', version: '9.0.0', type: 'native' }),
  );
  write(
    `${homeState}/state.json`,
    json({ enabled: home, method, homeId, appDir: app, stock: { id: 'x', version: '1.0.0' } }),
  );
  write(`${dir}/mounts`, '');
  write(`${bin}/id`, '#!/bin/sh\necho 0\n');
  // The installer must not need the TV's Node runtime.
  write(`${bin}/node`, '#!/bin/sh\nexit 127\n');
  write(`${bin}/stat`, `#!/bin/sh\necho ${stateOwner}\n`);
  write(`${bin}/sleep`, '#!/bin/sh\n/bin/sleep 0.01\n');
  write(
    `${bin}/luna-send`,
    `#!/bin/sh
printf '%s\\n' "$*" >> '${dir}/calls'
case "$*" in
  *dev/install*) echo '${install ? '{"returnValue":true,"state":"installed"}' : '{"returnValue":false,"errorText":"disk full"}'}' ;;
  *) echo '{"returnValue":${launch}}' ;;
esac
`,
  );
  write(`${app}/home.sh`, `#!/bin/sh\necho refresh >> '${dir}/calls'\nexit ${reapply ? 0 : 1}\n`);
  write(
    `${homeState}/home.sh`,
    `#!/bin/sh\necho restore >> '${dir}/calls'\nexit ${recover ? 0 : 1}\n`,
  );
  const script = source
    .replaceAll('/var/lib/luma-update', state)
    .replaceAll('/var/lib/luma-home', homeState)
    .replaceAll('/media/developer/apps/usr/palm/applications/com.eluz1ve.lumaplus', app)
    .replaceAll('/proc/mounts', `${dir}/mounts`)
    .replaceAll('/var/run/luma-home', `${dir}/marker`)
    .replaceAll('/tmp/luma-update-', `${dir}/luma-update-`)
    .replaceAll('find /tmp ', `find ${dir} `);
  write(`${dir}/installer.sh`, script);
  const sha = checksum ? createHash('sha256').update('test package').digest('hex') : '0'.repeat(64);
  try {
    const result = spawnSync(
      '/bin/sh',
      [`${dir}/installer.sh`, `${dir}/luma-update-${attempt}/package.ipk`, sha, '9.0.0', app, attempt],
      {
        env: { ...process.env, PATH: `${bin}:${process.env.PATH}` },
        encoding: 'utf8',
        timeout: 10000,
      },
    );
    assert.equal(result.signal, null, result.stderr);
    const calls = fs.existsSync(`${dir}/calls`) ? fs.readFileSync(`${dir}/calls`, 'utf8') : '';
    const resultFile = `${state}/result.${attempt}.json`;
    const downloads = fs.readdirSync(dir).filter((name) => name.startsWith('luma-update-'));
    if (!fs.existsSync(resultFile)) return { status: null, calls, exit: result.status, downloads };
    const status = JSON.parse(fs.readFileSync(resultFile, 'utf8'));
    assert.deepEqual(status, JSON.parse(fs.readFileSync(`${state}/result.json`, 'utf8')));
    const mode = fs.statSync(state).mode & 0o777;
    return { status, calls, exit: result.status, mode, downloads };
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
}

test('normal app update verifies the package and reports completion after launch', () => {
  const result = run();
  assert.equal(result.exit, 0);
  assert.equal(result.status.state, 'success');
  assert.match(result.calls, /launch.*com.eluz1ve.lumaplus/);
});
test('checksum and install failures publish retryable failure, never launch', () => {
  for (const options of [{ checksum: false }, { install: false }]) {
    const result = run(options);
    assert.equal(result.exit, 1);
    assert.equal(result.status.state, 'failed');
    assert.doesNotMatch(result.calls, /launch/);
  }
});
test('both Home methods refresh from the new package and fail visibly when reapply fails', () => {
  for (const method of ['executable', 'manifest']) {
    const success = run({ home: true, method });
    assert.equal(success.status.state, 'success');
    assert.match(success.calls, /refresh/);
    const failed = run({ home: true, method, reapply: false });
    assert.equal(failed.exit, 1);
    assert.equal(failed.status.state, 'failed');
    assert.match(failed.status.message, /LG Home restored/);
    assert.match(failed.calls, /restore[\s\S]*launch.*com.webos.app.home/);
    const unrecovered = run({ home: true, method, reapply: false, recover: false });
    assert.equal(unrecovered.status.state, 'failed');
    assert.doesNotMatch(unrecovered.calls, /launch/);
  }
});
test('a state directory left with the default mode is made private, not a silent stop', () => {
  // luma-helper's app updates create /var/lib/luma-update/apps, and with it this directory.
  const result = run({ stateMode: 0o755 });
  assert.equal(result.exit, 0);
  assert.equal(result.status?.state, 'success');
  assert.equal(result.mode, 0o700);
});
test('a state directory owned by someone else is never used', () => {
  const result = run({ stateMode: 0o755, stateOwner: 1000 });
  assert.equal(result.exit, 1);
  assert.equal(result.status, null);
  assert.doesNotMatch(result.calls, /dev\/install/);
});
test('the downloaded package does not outlive its verified copy', () => {
  // Installing can end Luma before it removes the download itself.
  const result = run();
  assert.equal(result.status.state, 'success');
  assert.deepEqual(result.downloads, []);
});
test('a package that failed its checksum stays for Luma to report and remove', () => {
  const result = run({ checksum: false });
  assert.deepEqual(result.downloads, [`luma-update-${'a'.repeat(32)}`]);
});
test('only abandoned download directories are swept', () => {
  const hours = new Date(Date.now() - 2 * 3600 * 1000);
  const recent = `luma-update-${'c'.repeat(32)}`;
  const link = `luma-update-${'d'.repeat(32)}`;
  let kept;
  const result = run({
    fixtures: ({ dir, download }) => {
      fs.utimesSync(download(`luma-update-${'b'.repeat(32)}`), hours, hours);
      fs.utimesSync(download('luma-update-notours'), hours, hours);
      download(recent);
      kept = `${dir}/target/package.ipk`;
      fs.mkdirSync(`${dir}/target`);
      fs.writeFileSync(kept, 'someone else');
      fs.symlinkSync(`${dir}/target`, `${dir}/${link}`);
      fs.lutimesSync(`${dir}/${link}`, hours, hours);
    },
  });
  assert.equal(result.status.state, 'success');
  assert.deepEqual(result.downloads.sort(), [recent, link, 'luma-update-notours'].sort());
});
test('a rejected launch is not reported as a successful update', () => {
  assert.equal(run({ launch: false }).status.state, 'failed');
});

test('fullhome remains the launch and recovery target after an update', () => {
  const homeId = 'com.webos.app.fullhome';
  const success = run({ home: true, homeId });
  assert.equal(success.status.state, 'success');
  assert.match(success.calls, /launch.*com.webos.app.fullhome/);
  assert.doesNotMatch(success.calls, /launch.*com.webos.app.home/);
  const failed = run({ home: true, homeId, reapply: false });
  assert.match(failed.status.message, /LG Home restored/);
  assert.match(failed.calls, /restore[\s\S]*launch.*com.webos.app.fullhome/);
});

test('pretty-printed manifests and old configurations without a target are read without Node', () => {
  for (const homeId of ['com.webos.app.fullhome', undefined]) {
    const result = run({ home: true, homeId, pretty: true });
    assert.equal(result.exit, 0, result.status.message);
    assert.equal(result.status.state, 'success');
    assert.match(result.calls, new RegExp(`launch.*${homeId ?? 'com.webos.app.home'}"`));
  }
});
