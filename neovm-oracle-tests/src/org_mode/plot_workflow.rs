use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_plot_options_tsv_quote_script_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-plot)
  (let* ((root (make-temp-file "org-plot-tsv" t))
         (data-file (expand-file-name "plot.dat" root))
         (png-file (expand-file-name "out.png" root)))
    (unwind-protect
        (with-temp-buffer
          (org-mode)
          (insert "#+PLOT: title:\"Demo Plot\" ind:1 deps:(2 3) with:lines\n")
          (insert "#+PLOT: set:\"grid\" set:\"key outside\" line:\"set xlabel 'Day'\" file:\"out.png\"\n")
          (insert "| Day | A | B |\n")
          (insert "|-----+---+---|\n")
          (insert "| <2026-05-27 Wed> | 2 | 3 |\n")
          (insert "| label \"x\" | 5 | 8 |\n")
          (goto-char (point-min))
          (let* ((params (org-plot/collect-options
                          (org-plot/collect-options
                           (copy-sequence org-plot/gnuplot-default-options))))
                 (table (progn
                          (org-plot/goto-nearest-table)
                          (delq 'hline (cdr (org-table-to-lisp)))))
                 (org-plot-timestamp-fmt "%Y/%m/%d")
                 (quoted (mapcar #'org-plot-quote-tsv-field
                                 '("<2026-05-27 Wed>" "label \"x\"" "42")))
                 (params (plist-put params :file png-file)))
            (org-plot/gnuplot-to-data table data-file params)
            (let* ((data (with-temp-buffer
                           (insert-file-contents data-file)
                           (buffer-string)))
                   (script (replace-regexp-in-string
                            (regexp-quote root)
                            "<root>"
                            (org-plot/gnuplot-script table data-file 3 params))))
              (list (plist-get params :title)
                    (plist-get params :ind)
                    (plist-get params :deps)
                    (plist-get params :with)
                    (reverse (plist-get params :set))
                    (plist-get params :line)
                    quoted
                    data
                    script))))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_plot_grid_data_map_ticks_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-plot)
  (let* ((root (make-temp-file "org-plot-grid" t))
         (data-file (expand-file-name "grid.dat" root))
         (table '(("North" "1" "2" "4")
                  ("South" "2" "4" "8")
                  ("West" "3" "9" "27")))
         (params '(:plot-type grid :ind 1 :deps (2 3 4) :map t :with pm3d)))
    (unwind-protect
        (let* ((row-labels
                (org-plot/gnuplot-to-grid-data table data-file params))
               (data (with-temp-buffer
                       (insert-file-contents data-file)
                       (buffer-string)))
               (params (plist-put params :ylabels row-labels))
               (script (replace-regexp-in-string
                        (regexp-quote root)
                        "<root>"
                        (org-plot/gnuplot-script table data-file 4 params)))
               (stats (org--plot/values-stats '(1 2 4 8) 0 10)))
          (list row-labels
                (plist-get stats :min)
                (plist-get stats :max)
                (plist-get stats :nice-range)
                (org--plot/sensible-tick-num table)
                data
                script))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_plot_radar_script_normalized_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-plot)
  (let* ((table '(("Speed" "4" "7")
                  ("Quality" "8" "5")
                  ("Cost" "2" "9")))
         (params '(:plot-type radar
                   :labels ("A" "B")
                   :ymin 0
                   :ymax 10
                   :ticks 5))
         (script (org-plot/gnuplot-script table "radar.dat" 3 params)))
    (list (not (null (string-match-p "spider plot" script)))
          (not (null (string-match-p "Speed" script)))
          (not (null (string-match-p "filledcurves" script)))
          (replace-regexp-in-string
           "/tmp/[^\"\n]+"
           "<tmp-file>"
           script))))"##,
    );
}
