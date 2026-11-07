// A build's release notes: the commits since the release before it, oldest first.
//
// `node tools/release-notes.mjs` reads this repository's releases with `gh` (GH_TOKEN in Actions)
// and prints the notes for HEAD. The release before is the newest published one whose commit
// HEAD descends from: a build's tag, or the commit a release before builds was made from.
import { execFileSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';

const git = (...args) => execFileSync('git', args, { encoding: 'utf8' }).trim();

/** Where a release was made from: its tag for a build, else the commit it was published at. */
export function releaseCommit(release, resolveTag) {
  if (release.draft) return null;
  if (/^build-\d{10}$/.test(release.tag_name)) return resolveTag(release.tag_name);
  return /^[0-9a-f]{40}$/.test(release.target_commitish ?? '') ? release.target_commitish : null;
}

/** The newest published release HEAD descends from, and its commit. */
export function previousRelease(releases, resolveTag, isAncestor) {
  const newest = [...releases].sort((a, b) =>
    (b.published_at ?? '').localeCompare(a.published_at ?? ''),
  );
  for (const release of newest) {
    const commit = releaseCommit(release, resolveTag);
    if (commit && isAncestor(commit)) return { release, commit };
  }
  return null;
}

/** Markdown for `subjects`, the commits from `base` (or the start, when null) to `head`. */
export function releaseNotes({ subjects, repository, base, head, previous }) {
  const lines = ['### Changes', ''];
  lines.push(...(subjects.length ? subjects.map((subject) => `- ${subject}`) : ['- Rebuilt']));
  if (base) {
    const range = `${base.slice(0, 7)}...${head.slice(0, 7)}`;
    lines.push(
      '',
      `Since ${previous}: [${range}](https://github.com/${repository}/compare/${range})`,
    );
  }
  return `${lines.join('\n')}\n`;
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) {
  const repository = process.env.GITHUB_REPOSITORY || 'eluz1ve/lg-luma-plus';
  const head = git('rev-parse', 'HEAD');
  const releases = JSON.parse(
    execFileSync('gh', ['api', `repos/${repository}/releases?per_page=100`], {
      encoding: 'utf8',
    }),
  );
  const found = previousRelease(
    releases,
    (tag) => {
      try {
        return git('rev-parse', `${tag}^{commit}`);
      } catch {
        return null;
      }
    },
    (commit) => {
      try {
        execFileSync('git', ['merge-base', '--is-ancestor', commit, head]);
        return true;
      } catch {
        return false;
      }
    },
  );
  const range = found ? `${found.commit}..${head}` : head;
  const subjects = git('log', '--no-merges', '--reverse', '--format=%s', range)
    .split('\n')
    .filter(Boolean);
  process.stdout.write(
    releaseNotes({
      subjects,
      repository,
      base: found?.commit ?? null,
      head,
      previous: found?.release.name || found?.release.tag_name,
    }),
  );
}
