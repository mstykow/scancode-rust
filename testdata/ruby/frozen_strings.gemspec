# frozen_string_literal: true

Gem::Specification.new do |s|
  s.name = "rubocop".freeze
  s.version = "1.50.0".freeze
  s.authors = ["Bozhidar Batsov".freeze, "Jonas Arvidsson".freeze, "Yuji Nakayama".freeze]
  s.email = "rubocop@googlegroups.com".freeze
  s.summary = "Automatic Ruby code style checking tool.".freeze
  s.description = "RuboCop is a Ruby code style checking and code formatting tool.".freeze
  s.homepage = "https://rubocop.org/".freeze
  s.license = "MIT".freeze

  s.add_runtime_dependency "json".freeze, "~> 2.3".freeze
  s.add_runtime_dependency "parallel".freeze, "~> 1.10".freeze
  s.add_development_dependency "bundler".freeze, ">= 1.15.0".freeze, "< 3.0.0".freeze
end
