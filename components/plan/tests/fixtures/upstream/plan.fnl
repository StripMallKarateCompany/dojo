;; A plan polled on GitHub tags, pinned to one series, matched by CPE and OSV.
{:name "tcl" :origin "karate" :version "8.6.16" :targets ["x86_64-linux" "aarch64-linux"]
 :version_source {:github "tcltk/tcl" :use_max_tag true :prefix "core-"
                  :include_regex "core-8-6-[0-9]+"}
 :advisory_ids {:cpe "cpe:2.3:a:tcl:tcl" :osv_git "https://github.com/tcltk/tcl"
                :tag_prefix "core-"}
 :fixes ["CVE-2024-1234" "GHSA-jfh8-c2jp-5v3q"]
 :on {:aarch64-linux {:deps ["karate/zlib"]}}}
