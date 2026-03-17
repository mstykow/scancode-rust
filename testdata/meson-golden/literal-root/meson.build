project(
  'demo-project',
  ['c', 'cpp'],
  version: '1.2.3',
  license: ['BSD-2-Clause', 'GPL-2.0-or-later'],
  license_files: ['COPYING', 'LICENSES/BSD'],
  meson_version: '>=0.47.0',
)

zlib_dep = dependency('zlib', version: '>=1.2.8')
threads_dep = dependency(
  'threads',
  required: false,
  method: 'system',
  modules: ['core', 'extra'],
  fallback: ['threads-subproject', 'threads_dep'],
  native: true,
)
dependency('libarchive', version: ['>=3.0.0', '<4.0.0'])
