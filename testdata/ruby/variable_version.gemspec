# frozen_string_literal: true

require "csv/version"

CSV::VERSION = "3.2.6"
ANOTHER_VERSION = "2.0.0"

Gem::Specification.new do |spec|
  spec.name = "csv"
  spec.version = CSV::VERSION
  spec.authors = ["James Edward Gray II", "Kouhei Sutou"]
  spec.email = ["james@grayproductions.net", "kou@cozmixng.org"]
  spec.summary = "CSV Reading and Writing"
  spec.description = "The CSV library provides a complete interface to CSV files and data."
  spec.homepage = "https://github.com/ruby/csv"
  spec.licenses = ["Ruby", "BSD-2-Clause"]

  spec.add_dependency "base64"
  spec.add_development_dependency "bundler"
end
