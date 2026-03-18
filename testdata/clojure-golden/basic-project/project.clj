(defproject org.example/sample "1.0.0"
  :description "Sample project"
  :url "https://example.org/sample"
  :license {:name "Eclipse Public License"
            :url "https://www.eclipse.org/legal/epl-v10.html"}
  :scm {:url "https://github.com/example/sample"}
  :dependencies [[org.clojure/clojure "1.11.1"]
                 [cheshire "5.12.0"]
                 ["ring/ring-core" "1.12.2" :classifier "tests"]]
  :profiles {:dev {:dependencies [[midje "1.10.10"]]}
             :provided {:dependencies [[javax.servlet/servlet-api "2.5"]]}
             :test {:dependencies [[lambdaisland/kaocha "1.91.1392"]]}})
