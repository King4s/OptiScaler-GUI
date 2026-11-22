'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const {
  findCanonicalIssue,
  findAttemptComment,
  shouldRunTest,
  stableReleasesFrom,
  selectNextUnclaimedRelease,
  latestStableRelease,
  statusForJobResult,
} = require('../upstream-release-watcher.cjs');

const bot = 'github-actions[bot]';
const marker = '<!-- optiscaler-real-upgrade-test:v1.2.3 -->';

test('selects the oldest exact-title issue authored by the workflow bot', () => {
  const issues = [
    {number: 5, title: 'OptiScaler v1.2.3 released — review for GUI compatibility', user: {login: 'human'}},
    {number: 9, title: 'OptiScaler v1.2.3 released — review for GUI compatibility', user: {login: bot}},
    {number: 2, title: 'OptiScaler update available: v1.2.3', user: {login: bot}},
    {number: 7, title: 'OptiScaler v1.2.3 released — review for GUI compatibility', user: {login: bot}, pull_request: {url: 'pull/7'}},
    {number: 4, title: 'OptiScaler v1.2.3 released — review for GUI compatibility', user: {login: bot}},
  ];
  assert.equal(findCanonicalIssue(issues, issues[1].title, bot).number, 4);
});

test('ignores forged marker text and comments not authored by the bot', () => {
  const comments = [
    {id: 1, user: {login: bot}, body: `Prior result ${marker}`},
    {id: 2, user: {login: 'human'}, body: marker},
    {id: 3, user: {login: bot}, body: `prefix ${marker}\nresult`},
  ];
  assert.equal(findAttemptComment(comments, marker, bot), undefined);
});

test('recognizes only a bot-authored marker at the start of a line', () => {
  const comments = [{id: 4, user: {login: bot}, body: `Run details\n${marker}\nfailed`}];
  assert.equal(findAttemptComment(comments, marker, bot).id, 4);
});

test('skips an existing attempt unless a forced rerun was requested', () => {
  const priorAttempt = {id: 4};
  assert.equal(shouldRunTest(priorAttempt, false), false);
  assert.equal(shouldRunTest(priorAttempt, true), true);
  assert.equal(shouldRunTest(undefined, false), true);
});

test('catches up stable releases oldest-first and excludes drafts, prereleases, and nightlies', () => {
  const releases = [
    {tag_name: 'v0.9.9', draft: false, prerelease: false},
    {tag_name: 'v1.0.0-nightly.1', draft: false, prerelease: true},
    {tag_name: 'v0.9.8', draft: false, prerelease: false},
    {tag_name: 'v0.9.7', draft: true, prerelease: false},
    {tag_name: 'v0.9.6-rc.1', draft: false, prerelease: false},
    {tag_name: 'v0.9.5', draft: false, prerelease: false},
    {tag_name: 'v0.9.4', draft: false, prerelease: false},
  ];
  const attempted = new Set(['v0.9.5']);

  assert.equal(selectNextUnclaimedRelease(releases, attempted).tag_name, 'v0.9.8');
  attempted.add('v0.9.8');
  assert.equal(selectNextUnclaimedRelease(releases, attempted).tag_name, 'v0.9.9');
  assert.deepEqual(stableReleasesFrom(releases).map(release => release.tag_name), [
    'v0.9.5', 'v0.9.8', 'v0.9.9',
  ]);
  assert.equal(latestStableRelease(releases).tag_name, 'v0.9.9');
});

test('maps every non-success job result, including cancellation, to failure', () => {
  assert.equal(statusForJobResult('success'), 'passed');
  for (const result of ['failure', 'cancelled', 'skipped', '']) {
    assert.equal(statusForJobResult(result), 'failed');
  }
});
