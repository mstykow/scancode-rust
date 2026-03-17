project(
  'guardrails',
  'c',
  version: version_value,
  license: license_value,
)

version_value = '9.9.9'
license_value = 'Apache-2.0'

direct_dep = dependency('libcurl', required: false)
computed_dep = dependency(dep_name)

if get_option('use_ssl')
  dependency('openssl', version: '>=3')
endif
