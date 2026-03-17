val orgName = "com.example"
val projectName = "demo-app"
val projectVersion = "1.2.3"
val catsVersion = "2.10.0"

ThisBuild / organization := orgName
ThisBuild / name := "fallback-name"
ThisBuild / version := projectVersion
ThisBuild / description := "Fallback description"
ThisBuild / organizationHomepage := Some(url("https://example.com/org"))

name := projectName
description := "Demo application"
homepage := Some(url("https://example.com/demo"))
licenses += "Apache-2.0" -> url("https://www.apache.org/licenses/LICENSE-2.0.txt")

libraryDependencies += "org.typelevel" %% "cats-core" % catsVersion
libraryDependencies += "org.scalatest" %% "scalatest" % "3.2.18" % Test
libraryDependencies ++= Seq(
  "javax.servlet" % "javax.servlet-api" % "4.0.1" % "provided",
  unsupportedDependency,
  "org.slf4j" % "slf4j-api" % "2.0.12"
)
