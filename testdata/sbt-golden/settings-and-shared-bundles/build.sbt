val commonDeps = Seq(
  "org.typelevel" %% "cats-core" % "2.10.0",
  "com.lihaoyi" %% "os-lib" % "0.10.0"
)
val testDeps = Seq(
  "org.scalatest" %% "scalatest" % "3.2.18" % Test
)
val commonSettings = Seq(
  organization := "com.example.bundle",
  version := "0.1.0",
  libraryDependencies ++= commonDeps,
  Test / libraryDependencies ++= testDeps
)

lazy val root = (project in file(".")).settings(
  commonSettings,
  name := "settings-demo",
  version := "2.0.0",
  organization := "com.example.root",
  description := "Bundle and settings demo",
  homepage := Some(url("https://example.com/settings-demo")),
  licenses += "Apache-2.0" -> url("https://www.apache.org/licenses/LICENSE-2.0.txt"),
  libraryDependencies += "ch.qos.logback" % "logback-classic" % "1.5.18"
)

lazy val core = project.settings(
  libraryDependencies += "org.ignored" %% "ignored" % "9.9.9"
)
