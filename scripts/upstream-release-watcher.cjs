'use strict';

function findCanonicalIssue(issues, title, botLogin) {
  return issues
    .filter(issue => !issue.pull_request && issue.title === title && issue.user?.login === botLogin)
    .sort((a, b) => a.number - b.number)[0];
}

function findAttemptComment(comments, marker, botLogin) {
  return comments.find(comment =>
    comment.user?.login === botLogin &&
    comment.body.split(/\r?\n/).some(line => line === marker)
  );
}

function shouldRunTest(priorAttempt, forceRerun) {
  return !priorAttempt || forceRerun;
}

function parseStableTag(tag) {
  const match = /^v?(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:\+[0-9A-Za-z.-]+)?$/.exec(tag);
  return match ? match.slice(1, 4).map(Number) : undefined;
}

function compareVersions(left, right) {
  for (let index = 0; index < 3; index += 1) {
    if (left[index] !== right[index]) return left[index] - right[index];
  }
  return 0;
}

function stableReleasesFrom(releases, minimumTag = 'v0.9.5') {
  const minimum = parseStableTag(minimumTag);
  if (!minimum) throw new TypeError(`Invalid minimum stable tag: ${minimumTag}`);

  return releases
    .filter(release => !release.draft && !release.prerelease)
    .map(release => ({release, version: parseStableTag(release.tag_name)}))
    .filter(item => item.version && compareVersions(item.version, minimum) >= 0)
    .sort((a, b) => compareVersions(a.version, b.version))
    .map(item => item.release);
}

function selectNextUnclaimedRelease(releases, claimedTags, minimumTag = 'v0.9.5') {
  return stableReleasesFrom(releases, minimumTag)
    .find(release => !claimedTags.has(release.tag_name));
}

function latestStableRelease(releases, minimumTag = 'v0.9.5') {
  return stableReleasesFrom(releases, minimumTag).at(-1);
}

function statusForJobResult(result) {
  return result === 'success' ? 'passed' : 'failed';
}

module.exports = {
  findCanonicalIssue,
  findAttemptComment,
  shouldRunTest,
  stableReleasesFrom,
  selectNextUnclaimedRelease,
  latestStableRelease,
  statusForJobResult,
};
