Gem::Specification.new do |spec|
  spec.name = "logstash-mixin-ecs_compatibility_support"
  spec.version = "1.0.0"
  spec.description = "This adapter provides an implementation of ECS-Compatibility mode that can be controlled at the plugin instance level."
  spec.summary = "ECS Compatibility Support"
  spec.licenses = ["Apache-2.0"]
  
  spec.add_dependency "logstash-core", ">= 6.0"
end
