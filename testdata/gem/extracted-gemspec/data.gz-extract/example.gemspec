Gem::Specification.new do |spec|
  spec.name = "example-gem"
  spec.version = "1.2.3"
  spec.authors = ["John Doe", "Jane Smith"]
  spec.email = ["john@example.com", "jane@example.com"]
  spec.summary = "A short summary of the gem"
  spec.description = "A longer description of the gem with more details"
  spec.homepage = "https://example.com/example-gem"
  spec.license = "MIT"

  spec.add_dependency "rails", "~> 5.0"
  spec.add_dependency "nokogiri", ">= 1.6"
  spec.add_development_dependency "rspec", "~> 3.0"
  spec.add_development_dependency "rubocop"
end
