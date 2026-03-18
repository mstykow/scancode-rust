name := "scoped-demo"
organization := "com.example"
version := "1.0.0"

Test / libraryDependencies += "org.scalatest" %% "scalatest" % "3.2.18"
Provided / libraryDependencies ++= Seq(
  "javax.servlet" % "javax.servlet-api" % "4.0.1",
  "com.example" % "provided-helper" % "2.0.0" % Test
)
Runtime / libraryDependencies += "ch.qos.logback" % "logback-classic" % "1.5.18"
Compile / libraryDependencies += "org.typelevel" %% "cats-core" % "2.10.0"

lazy val root = project.settings(
  Test / libraryDependencies += "org.nested" %% "ignored" % "9.9.9"
)
