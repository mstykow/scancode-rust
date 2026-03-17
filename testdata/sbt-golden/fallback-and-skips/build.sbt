val desc = "Alias description"
val depVersion = "1.0.0"

ThisBuild / name := "fallback-name"
organization := "com.fallback"
version := "0.1.0"
description := desc
organizationHomepage := Some(url("https://fallback.example.com/org"))
homepage := Some(url(homepageValue))
licenses += License.Apache

lazy val root = project.settings(
  libraryDependencies += "com.nested" % "ignored" % "9.9.9"
)

libraryDependencies += depGroup % "unresolved" % depVersion
libraryDependencies += "org.valid" % "artifact" % depVersion
