// Commit types allowed in this project (see CONTRIBUTING.md). The default
// conventional config also allows `revert`, which this project does not use.
export default {
  extends: ['@commitlint/config-conventional'],
  rules: {
    'type-enum': [
      2,
      'always',
      ['feat', 'fix', 'perf', 'refactor', 'docs', 'test', 'build', 'ci', 'style', 'chore'],
    ],
  },
};
