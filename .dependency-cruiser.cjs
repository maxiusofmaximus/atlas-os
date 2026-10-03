// Atlas OS — dependency-cruiser layer rules (RFC 20 Phase 9, sub-fase 9.3, M35).
//
// CSR-only frontend (RFC 25): `routes/` composes from `lib/`, `components/`
// import `stores/` — the dependency arrows point one way only. Run with
// `pnpm arch`.

module.exports = {
  forbidden: [
    {
      name: 'lib-may-not-import-routes',
      comment: 'The dependency points routes -> lib, never the other way (RFC 20 layer limits).',
      severity: 'error',
      from: { path: '^src/lib/' },
      to: { path: '^src/routes/' },
    },
    {
      name: 'stores-may-not-import-components',
      comment: 'Components import stores, never the reverse (RFC 20 layer limits).',
      severity: 'error',
      from: { path: '^src/lib/stores/' },
      to: { path: '^src/lib/components/' },
    },
    {
      name: 'components-may-not-import-routes',
      comment: 'Components are composed by routes, they never reach back up (RFC 20 layer limits).',
      severity: 'error',
      from: { path: '^src/lib/components/' },
      to: { path: '^src/routes/' },
    },
    {
      name: 'lib-no-node-only-modules',
      comment:
        'The frontend is CSR-only (RFC 25): lib/ may not touch Node-only builtins. Test files are exempt — vitest runs them in Node.',
      severity: 'error',
      from: { path: '^src/lib/', pathNot: '\\.test\\.ts$' },
      to: { dependencyTypes: ['core'] },
    },
  ],
  options: {
    doNotFollow: { path: 'node_modules' },
    tsConfig: { fileName: './tsconfig.json' },
    enhancedResolveOptions: {
      extensions: ['.js', '.ts', '.svelte'],
    },
  },
};
