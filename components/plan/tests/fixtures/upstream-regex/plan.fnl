;; A plan polled from a download page, matched by purl.
{:name "hello" :origin "karate" :version "1.0.0" :targets ["x86_64-linux"]
 :version_source {:regex "zlib-([0-9.]+)[.]tar[.]xz" :url "https://zlib.net/"
                  :exclude_regex "1[.]2[.].*"}
 :advisory_ids {:purl "pkg:generic/zlib"}}
