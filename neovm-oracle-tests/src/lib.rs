//! Oracle-backed Elisp parity tests.
#![allow(non_snake_case)]

#[path = "abbrev-comprehensive-patterns.rs"]
mod abbrev_comprehensive_patterns;
mod abs;
#[path = "accessible-keymaps-semantics.rs"]
mod accessible_keymaps_semantics;
#[path = "add-minor-mode-semantics.rs"]
mod add_minor_mode_semantics;
#[path = "add-to-list-semantics.rs"]
mod add_to_list_semantics;
#[path = "add-to-ordered-list-semantics.rs"]
mod add_to_ordered_list_semantics;
mod advice;
#[path = "advice-advanced.rs"]
mod advice_advanced;
#[path = "advice-comprehensive-patterns.rs"]
mod advice_comprehensive_patterns;
#[path = "advice-patterns-advanced.rs"]
mod advice_patterns_advanced;
#[path = "alist-get.rs"]
mod alist_get;
#[path = "alist-operations.rs"]
mod alist_operations;
#[path = "alist-operations-advanced-patterns.rs"]
mod alist_operations_advanced_patterns;
#[path = "alist-operations-comprehensive.rs"]
mod alist_operations_comprehensive;
mod r#and;
#[path = "append-nconc-semantics.rs"]
mod append_nconc_semantics;
mod apply;
#[path = "apply-advanced.rs"]
mod apply_advanced;
#[path = "apply-funcall-advanced.rs"]
mod apply_funcall_advanced;
#[path = "apply-funcall-comprehensive.rs"]
mod apply_funcall_comprehensive;
#[path = "apply-funcall-deep-edge-semantics.rs"]
mod apply_funcall_deep_edge_semantics;
#[path = "apply-funcall-edge-semantics.rs"]
mod apply_funcall_edge_semantics;
#[path = "apply-funcall-patterns.rs"]
mod apply_funcall_patterns;
#[path = "apply-lambda-comprehensive.rs"]
mod apply_lambda_comprehensive;
#[path = "apply-partially-semantics.rs"]
mod apply_partially_semantics;
#[path = "aref-aset.rs"]
mod aref_aset;
mod arithmetic;
#[path = "arithmetic-advanced.rs"]
mod arithmetic_advanced;
#[path = "arithmetic-bitwise-strict-edge-semantics.rs"]
mod arithmetic_bitwise_strict_edge_semantics;
#[path = "ash-logand-logior-patterns.rs"]
mod ash_logand_logior_patterns;
mod assoc;
#[path = "assoc-alist-comprehensive.rs"]
mod assoc_alist_comprehensive;
#[path = "assoc-assq-advanced.rs"]
mod assoc_assq_advanced;
#[path = "assoc-delete-semantics.rs"]
mod assoc_delete_semantics;
#[path = "assoc-member-strict-edge-semantics.rs"]
mod assoc_member_strict_edge_semantics;
mod assq;
#[path = "atimer-debug-availability-semantics.rs"]
mod atimer_debug_availability_semantics;
#[path = "autoload-semantics.rs"]
mod autoload_semantics;
mod backquote;
#[path = "backquote-advanced.rs"]
mod backquote_advanced;
#[path = "backquote-comprehensive-patterns.rs"]
mod backquote_comprehensive_patterns;
#[path = "backtrace-frame-semantics.rs"]
mod backtrace_frame_semantics;
#[path = "backup-file-name-strict-edge-semantics.rs"]
mod backup_file_name_strict_edge_semantics;
#[path = "base64-semantics.rs"]
mod base64_semantics;
#[path = "beginning-of-line.rs"]
mod beginning_of_line;
#[path = "bidi-string-semantics.rs"]
mod bidi_string_semantics;
#[path = "binding-forms-via-binary-semantics.rs"]
mod binding_forms_via_binary_semantics;
#[path = "binding-scoping-deep-edge-semantics.rs"]
mod binding_scoping_deep_edge_semantics;
mod bitwise;
#[path = "bool-vector-comprehensive.rs"]
mod bool_vector_comprehensive;
#[path = "bool-vector-edge-semantics.rs"]
mod bool_vector_edge_semantics;
#[path = "bool-vector-operations.rs"]
mod bool_vector_operations;
#[path = "bool-vector-syntax-strict-edge-semantics.rs"]
mod bool_vector_syntax_strict_edge_semantics;
#[path = "boolean-helper-semantics.rs"]
mod boolean_helper_semantics;
#[path = "bootstrap-library-require.rs"]
mod bootstrap_library_require;
#[path = "buffer-base-buffer-semantics.rs"]
mod buffer_base_buffer_semantics;
#[path = "buffer-edit-strict-edge-semantics.rs"]
mod buffer_edit_strict_edge_semantics;
#[path = "buffer-file-name-semantics.rs"]
mod buffer_file_name_semantics;
#[path = "buffer-hash-semantics.rs"]
mod buffer_hash_semantics;
#[path = "buffer-last-name-semantics.rs"]
mod buffer_last_name_semantics;
#[path = "buffer-line-statistics-semantics.rs"]
mod buffer_line_statistics_semantics;
#[path = "buffer-list-other-buffer-semantics.rs"]
mod buffer_list_other_buffer_semantics;
#[path = "buffer-local-default-value-edge-semantics.rs"]
mod buffer_local_default_value_edge_semantics;
#[path = "buffer-local-hook-unintern-strict-edge-semantics.rs"]
mod buffer_local_hook_unintern_strict_edge_semantics;
#[path = "buffer-local-set-state-semantics.rs"]
mod buffer_local_set_state_semantics;
#[path = "buffer-local-symbol-identity-semantics.rs"]
mod buffer_local_symbol_identity_semantics;
#[path = "buffer-local-toplevel-value-semantics.rs"]
mod buffer_local_toplevel_value_semantics;
#[path = "buffer-local-variable-patterns.rs"]
mod buffer_local_variable_patterns;
#[path = "buffer-match-semantics.rs"]
mod buffer_match_semantics;
#[path = "buffer-mgmt-strict-edge-semantics.rs"]
mod buffer_mgmt_strict_edge_semantics;
#[path = "buffer-modification-comprehensive.rs"]
mod buffer_modification_comprehensive;
#[path = "buffer-modified-p-semantics.rs"]
mod buffer_modified_p_semantics;
#[path = "buffer-movement-strict-edge-semantics.rs"]
mod buffer_movement_strict_edge_semantics;
#[path = "buffer-multi-operations.rs"]
mod buffer_multi_operations;
#[path = "buffer-name.rs"]
mod buffer_name;
#[path = "buffer-operations.rs"]
mod buffer_operations;
#[path = "buffer-operations-advanced.rs"]
mod buffer_operations_advanced;
#[path = "buffer-ops-strict-edge-semantics.rs"]
mod buffer_ops_strict_edge_semantics;
#[path = "buffer-position.rs"]
mod buffer_position;
#[path = "buffer-position-patterns.rs"]
mod buffer_position_patterns;
#[path = "buffer-search-comprehensive.rs"]
mod buffer_search_comprehensive;
#[path = "buffer-search-replace-strict-edge-semantics.rs"]
mod buffer_search_replace_strict_edge_semantics;
#[path = "buffer-string.rs"]
mod buffer_string;
#[path = "buffer-substring.rs"]
mod buffer_substring;
#[path = "buffer-substring-advanced.rs"]
mod buffer_substring_advanced;
#[path = "buffer-text-deep-edge-semantics.rs"]
mod buffer_text_deep_edge_semantics;
#[path = "buffer-undo-posix-semantics.rs"]
mod buffer_undo_posix_semantics;
#[path = "bufferp-semantics.rs"]
mod bufferp_semantics;
#[path = "button-semantics.rs"]
mod button_semantics;
#[path = "byte-operations-comprehensive.rs"]
mod byte_operations_comprehensive;
#[path = "call-shell-region-semantics.rs"]
mod call_shell_region_semantics;
#[path = "called-interactively-semantics.rs"]
mod called_interactively_semantics;
#[path = "car-cdr-combinations.rs"]
mod car_cdr_combinations;
#[path = "car-safe.rs"]
mod car_safe;
#[path = "case-convert-string-char-strict-edge-semantics.rs"]
mod case_convert_string_char_strict_edge_semantics;
mod r#catch;
#[path = "catch-throw-advanced.rs"]
mod catch_throw_advanced;
#[path = "catch-throw-comprehensive.rs"]
mod catch_throw_comprehensive;
#[path = "catch-throw-edge-semantics.rs"]
mod catch_throw_edge_semantics;
#[path = "catch-throw-patterns.rs"]
mod catch_throw_patterns;
#[path = "change-group-semantics.rs"]
mod change_group_semantics;
#[path = "char-after.rs"]
mod char_after;
#[path = "char-before-operations.rs"]
mod char_before_operations;
#[path = "char-bool-math-deep-edge-semantics.rs"]
mod char_bool_math_deep_edge_semantics;
#[path = "char-byte-strict-edge-semantics.rs"]
mod char_byte_strict_edge_semantics;
#[path = "char-literal.rs"]
mod char_literal;
#[path = "char-literal-advanced.rs"]
mod char_literal_advanced;
#[path = "char-operations.rs"]
mod char_operations;
#[path = "char-operations-comprehensive.rs"]
mod char_operations_comprehensive;
#[path = "char-syntax-advanced.rs"]
mod char_syntax_advanced;
#[path = "char-table.rs"]
mod char_table;
#[path = "char-table-advanced.rs"]
mod char_table_advanced;
#[path = "char-table-comprehensive-patterns.rs"]
mod char_table_comprehensive_patterns;
#[path = "char-table-extra-slot.rs"]
mod char_table_extra_slot;
#[path = "char-table-patterns.rs"]
mod char_table_patterns;
#[path = "char-table-range-advanced.rs"]
mod char_table_range_advanced;
#[path = "char-to-string.rs"]
mod char_to_string;
#[path = "char-width-advanced.rs"]
mod char_width_advanced;
mod charset;
#[path = "charset-advanced.rs"]
mod charset_advanced;
#[path = "cl-defstruct-comprehensive.rs"]
mod cl_defstruct_comprehensive;
#[path = "cl-lib-comprehensive.rs"]
mod cl_lib_comprehensive;
#[path = "cl-lib-patterns.rs"]
mod cl_lib_patterns;
#[path = "cl-lib-patterns-advanced.rs"]
mod cl_lib_patterns_advanced;
#[path = "cl-loop-advanced-patterns.rs"]
mod cl_loop_advanced_patterns;
#[path = "cl-loop-comprehensive.rs"]
mod cl_loop_comprehensive;
#[path = "cl-loop-patterns.rs"]
mod cl_loop_patterns;
#[path = "clear-string-semantics.rs"]
mod clear_string_semantics;
mod closure;
#[path = "closure-advanced.rs"]
mod closure_advanced;
#[path = "closure-capture-patterns.rs"]
mod closure_capture_patterns;
#[path = "closure-lexical-comprehensive.rs"]
mod closure_lexical_comprehensive;
mod coding;
#[path = "coding-advanced.rs"]
mod coding_advanced;
#[path = "coding-metadata.rs"]
mod coding_metadata;
#[path = "coding-string.rs"]
mod coding_string;
#[path = "coding-string-advanced.rs"]
mod coding_string_advanced;
#[path = "coding-system-comprehensive.rs"]
mod coding_system_comprehensive;
#[path = "coding-system-put-advanced.rs"]
mod coding_system_put_advanced;
#[path = "coding-textprop-strict-edge-semantics.rs"]
mod coding_textprop_strict_edge_semantics;
#[path = "coding-utf8-test-availability-semantics.rs"]
mod coding_utf8_test_availability_semantics;
mod combination;
mod combination_a_star_search;
mod combination_abstract_algebra;
mod combination_abstract_algebra_advanced;
mod combination_abstract_algebra_groups;
mod combination_abstract_data_types;
mod combination_abstract_domain;
mod combination_abstract_interpretation;
mod combination_abstract_interpretation_advanced;
mod combination_abstract_interpreter;
mod combination_abstract_machine;
mod combination_abstract_machine_cek;
mod combination_abstract_machine_secd;
mod combination_abstract_machine_warren;
mod combination_abstract_machines;
mod combination_abstract_rewriting;
mod combination_abstract_set;
mod combination_abstract_syntax;
mod combination_actor_model;
mod combination_advanced;
mod combination_advanced_error_handling;
mod combination_algorithm_challenges;
mod combination_algorithms;
mod combination_alist_patterns;
mod combination_automata_theory;
mod combination_automata_theory_advanced;
mod combination_automaton_cellular;
mod combination_automaton_patterns;
mod combination_avl_tree;
mod combination_b_tree;
mod combination_binomial_heap;
mod combination_bitset_operations;
mod combination_blockchain_sim;
mod combination_bloom_filter;
mod combination_bloom_filter_advanced;
mod combination_buffer_advanced;
mod combination_buffer_algorithms;
mod combination_buffer_editing;
mod combination_buffer_processing;
mod combination_buffer_text_processing;
mod combination_bytecode_interpreter;
mod combination_bytevector_ops;
mod combination_cache_strategies;
mod combination_calculator_repl;
mod combination_category_theory;
mod combination_category_theory_advanced;
mod combination_channel_patterns;
mod combination_church_encoding;
mod combination_closures;
mod combination_closures_advanced;
mod combination_collections;
mod combination_compiler_codegen;
mod combination_compiler_optimizer;
mod combination_compiler_patterns;
mod combination_compiler_register_allocator;
mod combination_compiler_ssa;
mod combination_complex;
mod combination_compression;
mod combination_computer_algebra;
mod combination_concurrent_patterns;
mod combination_config_system;
mod combination_consensus;
mod combination_consistent_hashing;
mod combination_constraint_logic;
mod combination_constraint_logic_advanced;
mod combination_constraint_propagation;
mod combination_constraint_solver_advanced;
mod combination_constraint_solving;
mod combination_continuation_passing;
mod combination_contract_system;
mod combination_control_flow;
mod combination_coroutine_patterns;
mod combination_cps_transform;
mod combination_cps_transform_advanced;
mod combination_cryptography;
mod combination_cryptography_advanced;
mod combination_csp_solver;
mod combination_dag_operations;
mod combination_data_structures;
mod combination_data_structures_advanced;
mod combination_data_transformations;
mod combination_database_ops;
mod combination_database_patterns;
mod combination_database_relational;
mod combination_dataflow_analysis;
mod combination_dataflow_analysis_advanced;
mod combination_datalog;
mod combination_decision_tree;
mod combination_deductive_database;
mod combination_dependency_resolver;
mod combination_deque_operations;
mod combination_deque_operations_advanced;
mod combination_design_patterns;
mod combination_diff_algorithm;
mod combination_difference_list;
mod combination_disjoint_set;
mod combination_distributed_system_sim;
mod combination_dynamic_programming;
mod combination_earley_parser;
mod combination_effect_system;
mod combination_elisp_idioms;
mod combination_encoding_algorithms;
mod combination_error_handling;
mod combination_event_driven;
mod combination_event_sourcing;
mod combination_event_system;
mod combination_expression_compiler;
mod combination_expression_evaluator;
mod combination_expression_tree;
mod combination_fenwick_tree;
mod combination_finger_tree;
mod combination_finite_automata;
mod combination_formal_language;
mod combination_functional;
mod combination_functional_advanced;
mod combination_functional_composition;
mod combination_functional_lens;
mod combination_functional_programming;
mod combination_functional_reactive;
mod combination_game_theory;
mod combination_garbage_collector_sim;
mod combination_genetic_algorithm;
mod combination_genetic_programming;
mod combination_graph_algorithm_advanced;
mod combination_graph_algorithms;
mod combination_graph_coloring;
mod combination_graph_patterns;
mod combination_graph_shortest_path;
mod combination_graph_theory_advanced;
mod combination_graph_traversal;
mod combination_hash_algorithms;
mod combination_heap_datastructure;
mod combination_higher_order;
mod combination_huffman_coding;
mod combination_huffman_coding_advanced;
mod combination_image_processing;
mod combination_immutable_data;
mod combination_information_retrieval;
mod combination_interpreter_advanced;
mod combination_interpreter_advanced2;
mod combination_interpreter_calculus;
mod combination_interpreter_patterns;
mod combination_interpreter_register_vm;
mod combination_interpreters;
mod combination_interval_arithmetic;
mod combination_interval_tree;
mod combination_interval_tree_advanced;
mod combination_iterative_algorithms;
mod combination_iterator_patterns;
mod combination_json_processor;
mod combination_kd_tree;
mod combination_knuth_morris_pratt;
mod combination_lambda_calculus;
mod combination_lambda_calculus_advanced;
mod combination_lattice_theory;
mod combination_lazy_evaluation;
mod combination_lexer_patterns;
mod combination_linear_algebra;
mod combination_linked_list_ops;
mod combination_list_algorithms;
mod combination_logic_engine;
mod combination_logic_programming;
mod combination_logic_programming_advanced;
mod combination_logic_puzzles;
mod combination_lru_cache;
mod combination_macro_patterns;
mod combination_markov_chain;
mod combination_markup_parser;
mod combination_mathematical_structures;
mod combination_matrix_computation;
mod combination_matrix_decomposition;
mod combination_matrix_math;
mod combination_matrix_operations;
mod combination_matrix_operations_advanced;
mod combination_memo_table;
mod combination_memory_allocator;
mod combination_metaprogramming;
mod combination_mini_languages;
mod combination_minimax;
mod combination_minimax_advanced;
mod combination_model_checker;
mod combination_monad_patterns;
mod combination_monoid_patterns;
mod combination_natural_language;
mod combination_network_flow;
mod combination_network_protocol_sim;
mod combination_neural_network_sim;
mod combination_numeric_algorithms;
mod combination_numeric_patterns;
mod combination_object_system;
mod combination_oop_patterns;
mod combination_operating_system_scheduler;
mod combination_operating_system_sim;
mod combination_optimization_algorithm;
mod combination_parser_combinator_advanced;
mod combination_parser_combinators;
mod combination_parser_earley_advanced;
mod combination_parser_json;
mod combination_parser_ll1;
mod combination_parser_lr;
mod combination_parser_peg;
mod combination_parser_recursive_descent;
mod combination_parser_state_machine;
mod combination_parsing;
mod combination_pattern_language;
mod combination_pattern_matching;
mod combination_patterns;
mod combination_persistent_data;
mod combination_persistent_data_advanced;
mod combination_persistent_queue;
mod combination_petri_net;
mod combination_petri_net_advanced;
mod combination_physics_sim;
mod combination_polynomial_arithmetic;
mod combination_pratt_parser;
mod combination_priority_queue;
mod combination_probability_stats;
mod combination_problem_solving;
mod combination_process_algebra;
mod combination_promise_patterns;
mod combination_proof_assistant;
mod combination_property_list_patterns;
mod combination_protocol_fsm;
mod combination_protocol_implementations;
mod combination_protocol_state_advanced;
mod combination_protocol_verification;
mod combination_query_language;
mod combination_queue_stack;
mod combination_railroad_diagram;
mod combination_ray_tracer;
mod combination_reactive_patterns;
mod combination_reactive_system;
mod combination_real_world;
mod combination_real_world_elisp;
mod combination_recursion;
mod combination_recursive_descent_advanced;
mod combination_red_black_tree;
mod combination_red_black_tree_advanced;
mod combination_red_black_tree_comprehensive;
mod combination_regex_engine;
mod combination_regex_nfa;
mod combination_register_machine;
mod combination_register_machine_advanced;
mod combination_ring_buffer;
mod combination_rope_datastructure;
mod combination_rope_operations;
mod combination_sat_solver;
mod combination_sat_solver_advanced;
mod combination_scheduling;
mod combination_scheduling_advanced;
mod combination_segment_tree;
mod combination_serialization;
mod combination_set_operations;
mod combination_signal_processing;
mod combination_signal_processing_advanced;
mod combination_simulation;
mod combination_skip_list;
mod combination_sorting;
mod combination_sparse_matrix;
mod combination_splay_tree;
mod combination_state_machines;
mod combination_stream_processing;
mod combination_string_advanced;
mod combination_string_algorithms;
mod combination_string_algorithms_advanced;
mod combination_string_formatting;
mod combination_string_interning;
mod combination_string_parsing;
mod combination_suffix_array;
mod combination_suffix_tree;
mod combination_symbolic_differentiation;
mod combination_symbolic_execution;
mod combination_symbolic_math;
mod combination_symbolic_math_advanced;
mod combination_term_rewriting;
mod combination_term_rewriting_advanced;
mod combination_text_analysis;
mod combination_text_formatting;
mod combination_text_processing;
mod combination_text_templating;
mod combination_theorem_prover;
mod combination_topological_sort;
mod combination_topological_sort_advanced;
mod combination_treap;
mod combination_tree_algorithms;
mod combination_trie_advanced;
mod combination_trie_compressed;
mod combination_trie_datastructure;
mod combination_trie_router;
mod combination_type_checker;
mod combination_type_inference;
mod combination_type_inference_hm;
mod combination_type_system_advanced;
mod combination_type_systems;
mod combination_undo_system;
mod combination_unification;
mod combination_union_find;
mod combination_validation;
mod combination_verification_system;
mod combination_virtual_machine_advanced;
mod combination_workflow;
mod combination_zipper_datastructure;
#[path = "combine-and-quote-strings-semantics.rs"]
mod combine_and_quote_strings_semantics;
#[path = "command-modes.rs"]
mod command_modes;
#[path = "commandp-functionp-advanced.rs"]
mod commandp_functionp_advanced;
pub(crate) mod common;
#[path = "compare-strings.rs"]
mod compare_strings;
#[path = "compare-strings-advanced.rs"]
mod compare_strings_advanced;
#[path = "compare-strings-comprehensive.rs"]
mod compare_strings_comprehensive;
mod comparison;
#[path = "comparison-advanced.rs"]
mod comparison_advanced;
#[path = "completion-basic-semantics.rs"]
mod completion_basic_semantics;
#[path = "concat-extended.rs"]
mod concat_extended;
#[path = "concat-extended-advanced.rs"]
mod concat_extended_advanced;
#[path = "concat-nconc-final-strict-edge-semantics.rs"]
mod concat_nconc_final_strict_edge_semantics;
mod cond;
#[path = "cond-advanced.rs"]
mod cond_advanced;
#[path = "cond-comprehensive-patterns.rs"]
mod cond_comprehensive_patterns;
#[path = "condition-case.rs"]
mod condition_case;
#[path = "condition-case-advanced2.rs"]
mod condition_case_advanced2;
#[path = "condition-case-comprehensive.rs"]
mod condition_case_comprehensive;
#[path = "condition-case-error-data-strict-semantics.rs"]
mod condition_case_error_data_strict_semantics;
#[path = "condition-case-error-data-via-binary-semantics.rs"]
mod condition_case_error_data_via_binary_semantics;
#[path = "condition-case-extended.rs"]
mod condition_case_extended;
#[path = "condition-case-nested-edge-semantics.rs"]
mod condition_case_nested_edge_semantics;
#[path = "condition-case-patterns.rs"]
mod condition_case_patterns;
#[path = "condition-case-patterns-advanced.rs"]
mod condition_case_patterns_advanced;
#[path = "condition-case-unless-debug-semantics.rs"]
mod condition_case_unless_debug_semantics;
#[path = "conditional-binding-macros-semantics.rs"]
mod conditional_binding_macros_semantics;
#[path = "cons-list-dotted-comprehensive.rs"]
mod cons_list_dotted_comprehensive;
#[path = "cons-mutation-edge-semantics.rs"]
mod cons_mutation_edge_semantics;
#[path = "conversion-format-semantics.rs"]
mod conversion_format_semantics;
#[path = "copy-alist.rs"]
mod copy_alist;
#[path = "copy-alist-advanced.rs"]
mod copy_alist_advanced;
#[path = "copy-alist-sequence-patterns.rs"]
mod copy_alist_sequence_patterns;
#[path = "copy-file-strict-edge-semantics.rs"]
mod copy_file_strict_edge_semantics;
#[path = "copy-keymap-advanced.rs"]
mod copy_keymap_advanced;
#[path = "copy-read-strict-edge-semantics.rs"]
mod copy_read_strict_edge_semantics;
#[path = "copy-sequence.rs"]
mod copy_sequence;
#[path = "copy-sequence-advanced.rs"]
mod copy_sequence_advanced;
#[path = "copy-sequence-hash-alist-deep-semantics.rs"]
mod copy_sequence_hash_alist_deep_semantics;
#[path = "copy-sequence-semantics.rs"]
mod copy_sequence_semantics;
#[path = "copy-syntax-ppss-strict-edge-semantics.rs"]
mod copy_syntax_ppss_strict_edge_semantics;
#[path = "copy-syntax-table-advanced.rs"]
mod copy_syntax_table_advanced;
#[path = "copy-tree-semantics.rs"]
mod copy_tree_semantics;
#[path = "count-lines-advanced.rs"]
mod count_lines_advanced;
#[path = "count-lines-patterns.rs"]
mod count_lines_patterns;
mod coverage;
mod coverage_manifest;
#[path = "current-buffer.rs"]
mod current_buffer;
#[path = "current-column-advanced.rs"]
mod current_column_advanced;
#[path = "current-idle-time-message-semantics.rs"]
mod current_idle_time_message_semantics;
#[path = "cxxr-semantics.rs"]
mod cxxr_semantics;
#[path = "data-construction-strict-edge-semantics.rs"]
mod data_construction_strict_edge_semantics;
#[path = "dbus-inhibitor-lock-semantics.rs"]
mod dbus_inhibitor_lock_semantics;
#[path = "decode-char-encode-char-advanced.rs"]
mod decode_char_encode_char_advanced;
#[path = "defalias-advanced.rs"]
mod defalias_advanced;
#[path = "defalias-fset-patterns.rs"]
mod defalias_fset_patterns;
#[path = "default-boundp-semantics.rs"]
mod default_boundp_semantics;
#[path = "default-file-modes-strict-edge-semantics.rs"]
mod default_file_modes_strict_edge_semantics;
#[path = "default-toplevel-value-semantics.rs"]
mod default_toplevel_value_semantics;
#[path = "define-error-semantics.rs"]
mod define_error_semantics;
#[path = "define-key-advanced.rs"]
mod define_key_advanced;
#[path = "define-key-after-semantics.rs"]
mod define_key_after_semantics;
#[path = "define-prefix-command-semantics.rs"]
mod define_prefix_command_semantics;
#[path = "defmacro-advanced.rs"]
mod defmacro_advanced;
#[path = "defmacro-comprehensive-patterns.rs"]
mod defmacro_comprehensive_patterns;
#[path = "defmacro-macroexpand.rs"]
mod defmacro_macroexpand;
#[path = "defmacro-patterns.rs"]
mod defmacro_patterns;
#[path = "defun-comprehensive-patterns.rs"]
mod defun_comprehensive_patterns;
mod defvar;
#[path = "defvar-advanced.rs"]
mod defvar_advanced;
#[path = "defvar-setq-comprehensive.rs"]
mod defvar_setq_comprehensive;
#[path = "delayed-warning-semantics.rs"]
mod delayed_warning_semantics;
#[path = "delete-and-extract-advanced.rs"]
mod delete_and_extract_advanced;
#[path = "delete-char-patterns.rs"]
mod delete_char_patterns;
#[path = "delete-consecutive-dups-semantics.rs"]
mod delete_consecutive_dups_semantics;
#[path = "delete-dups-semantics.rs"]
mod delete_dups_semantics;
#[path = "delete-file-strict-edge-semantics.rs"]
mod delete_file_strict_edge_semantics;
#[path = "delete-member-assoc-deep-interaction-semantics.rs"]
mod delete_member_assoc_deep_interaction_semantics;
#[path = "delete-operations.rs"]
mod delete_operations;
#[path = "delete-operations-advanced.rs"]
mod delete_operations_advanced;
#[path = "delete-region.rs"]
mod delete_region;
#[path = "delete-region-advanced.rs"]
mod delete_region_advanced;
#[path = "delete-region-comprehensive.rs"]
mod delete_region_comprehensive;
#[path = "delete-remove-semantics.rs"]
mod delete_remove_semantics;
mod delq;
#[path = "derived-mode-semantics.rs"]
mod derived_mode_semantics;
#[path = "directory-abbrev-strict-edge-semantics.rs"]
mod directory_abbrev_strict_edge_semantics;
#[path = "directory-empty-strict-edge-semantics.rs"]
mod directory_empty_strict_edge_semantics;
#[path = "directory-files-and-attributes-strict-edge-semantics.rs"]
mod directory_files_and_attributes_strict_edge_semantics;
#[path = "directory-files-recursively-strict-edge-semantics.rs"]
mod directory_files_recursively_strict_edge_semantics;
#[path = "directory-files-strict-edge-semantics.rs"]
mod directory_files_strict_edge_semantics;
#[path = "directory-name-transform-strict-edge-semantics.rs"]
mod directory_name_transform_strict_edge_semantics;
#[path = "directory-wrapper-strict-edge-semantics.rs"]
mod directory_wrapper_strict_edge_semantics;
#[path = "dlet-semantics.rs"]
mod dlet_semantics;
#[path = "docstring-format-semantics.rs"]
mod docstring_format_semantics;
mod dolist;
#[path = "dolist-dotimes-advanced.rs"]
mod dolist_dotimes_advanced;
#[path = "dolist-dotimes-comprehensive.rs"]
mod dolist_dotimes_comprehensive;
mod dotimes;
#[path = "dynamic-binding.rs"]
mod dynamic_binding;
#[path = "dynamic-binding-advanced.rs"]
mod dynamic_binding_advanced;
#[path = "dynamic-binding-comprehensive.rs"]
mod dynamic_binding_comprehensive;
#[path = "eieio-comprehensive.rs"]
mod eieio_comprehensive;
#[path = "elt-aref-aset-patterns.rs"]
mod elt_aref_aset_patterns;
#[path = "encode-decode-coding-advanced.rs"]
mod encode_decode_coding_advanced;
#[path = "encoding-base64-via-binary-semantics.rs"]
mod encoding_base64_via_binary_semantics;
#[path = "end-of-line.rs"]
mod end_of_line;
#[path = "eq-eql-equal-edge-semantics.rs"]
mod eq_eql_equal_edge_semantics;
mod equality;
#[path = "equality-advanced.rs"]
mod equality_advanced;
#[path = "equality-hash-semantics.rs"]
mod equality_hash_semantics;
#[path = "erase-buffer-advanced.rs"]
mod erase_buffer_advanced;
#[path = "erase-buffer-patterns.rs"]
mod erase_buffer_patterns;
#[path = "error-flow-loop-via-binary-semantics.rs"]
mod error_flow_loop_via_binary_semantics;
#[path = "error-handling-comprehensive.rs"]
mod error_handling_comprehensive;
#[path = "error-handling-patterns.rs"]
mod error_handling_patterns;
#[path = "error-handling-patterns-advanced.rs"]
mod error_handling_patterns_advanced;
#[path = "error-types-comprehensive.rs"]
mod error_types_comprehensive;
mod eval;
#[path = "eval-advanced.rs"]
mod eval_advanced;
#[path = "eval-advanced-2.rs"]
mod eval_advanced_2;
#[path = "eval-apply-advanced.rs"]
mod eval_apply_advanced;
#[path = "eval-apply-strict-edge-semantics.rs"]
mod eval_apply_strict_edge_semantics;
#[path = "eval-buffer-region-semantics.rs"]
mod eval_buffer_region_semantics;
#[path = "eval-comprehensive-patterns.rs"]
mod eval_comprehensive_patterns;
#[path = "eval-regexp-pcase-edge-semantics.rs"]
mod eval_regexp_pcase_edge_semantics;
#[path = "event-convert-advanced.rs"]
mod event_convert_advanced;
mod event_convert_list;
#[path = "event-modifier-symbol-semantics.rs"]
mod event_modifier_symbol_semantics;
#[path = "event-posn-accessor-semantics.rs"]
mod event_posn_accessor_semantics;
#[path = "event-predicate-semantics.rs"]
mod event_predicate_semantics;
#[path = "executable-find-strict-edge-semantics.rs"]
mod executable_find_strict_edge_semantics;
#[path = "expand-file-name-strict-edge-semantics.rs"]
mod expand_file_name_strict_edge_semantics;
#[path = "expt-sqrt-log-patterns.rs"]
mod expt_sqrt_log_patterns;
#[path = "field-at-pos-semantics.rs"]
mod field_at_pos_semantics;
#[path = "file-access-ownership-strict-edge-semantics.rs"]
mod file_access_ownership_strict_edge_semantics;
#[path = "file-acl-strict-edge-semantics.rs"]
mod file_acl_strict_edge_semantics;
#[path = "file-attributes-strict-edge-semantics.rs"]
mod file_attributes_strict_edge_semantics;
#[path = "file-chase-links-strict-edge-semantics.rs"]
mod file_chase_links_strict_edge_semantics;
#[path = "file-expand-wildcards-strict-edge-semantics.rs"]
mod file_expand_wildcards_strict_edge_semantics;
#[path = "file-has-changed-strict-edge-semantics.rs"]
mod file_has_changed_strict_edge_semantics;
#[path = "file-in-directory-strict-edge-semantics.rs"]
mod file_in_directory_strict_edge_semantics;
#[path = "file-io-region-strict-edge-semantics.rs"]
mod file_io_region_strict_edge_semantics;
#[path = "file-link-creation-strict-edge-semantics.rs"]
mod file_link_creation_strict_edge_semantics;
#[path = "file-lock-strict-edge-semantics.rs"]
mod file_lock_strict_edge_semantics;
#[path = "file-mode-symbolic-strict-edge-semantics.rs"]
mod file_mode_symbolic_strict_edge_semantics;
#[path = "file-modes-strict-edge-semantics.rs"]
mod file_modes_strict_edge_semantics;
#[path = "file-name-absolute-strict-edge-semantics.rs"]
mod file_name_absolute_strict_edge_semantics;
#[path = "file-name-case-insensitive-strict-edge-semantics.rs"]
mod file_name_case_insensitive_strict_edge_semantics;
#[path = "file-name-completion-strict-edge-semantics.rs"]
mod file_name_completion_strict_edge_semantics;
#[path = "file-name-component-strict-edge-semantics.rs"]
mod file_name_component_strict_edge_semantics;
#[path = "file-name-concat-strict-edge-semantics.rs"]
mod file_name_concat_strict_edge_semantics;
#[path = "file-name-quote-strict-edge-semantics.rs"]
mod file_name_quote_strict_edge_semantics;
#[path = "file-name-semantics.rs"]
mod file_name_semantics;
#[path = "file-newer-than-strict-edge-semantics.rs"]
mod file_newer_than_strict_edge_semantics;
#[path = "file-nlinks-strict-edge-semantics.rs"]
mod file_nlinks_strict_edge_semantics;
#[path = "file-predicate-strict-edge-semantics.rs"]
mod file_predicate_strict_edge_semantics;
#[path = "file-relative-name-strict-edge-semantics.rs"]
mod file_relative_name_strict_edge_semantics;
#[path = "file-remote-local-strict-edge-semantics.rs"]
mod file_remote_local_strict_edge_semantics;
#[path = "file-repository-version-semantics.rs"]
mod file_repository_version_semantics;
#[path = "file-selinux-strict-edge-semantics.rs"]
mod file_selinux_strict_edge_semantics;
#[path = "file-size-human-readable-strict-edge-semantics.rs"]
mod file_size_human_readable_strict_edge_semantics;
#[path = "file-times-strict-edge-semantics.rs"]
mod file_times_strict_edge_semantics;
#[path = "file-truename-equal-strict-edge-semantics.rs"]
mod file_truename_equal_strict_edge_semantics;
#[path = "fillarray-advanced.rs"]
mod fillarray_advanced;
#[path = "fillarray-operations.rs"]
mod fillarray_operations;
#[path = "final-gaps-strict-edge-semantics.rs"]
mod final_gaps_strict_edge_semantics;
#[path = "find-backup-file-name-strict-edge-semantics.rs"]
mod find_backup_file_name_strict_edge_semantics;
#[path = "find-buffer-semantics.rs"]
mod find_buffer_semantics;
#[path = "flatten-tree-semantics.rs"]
mod flatten_tree_semantics;
#[path = "float-nan-misc-edge-semantics.rs"]
mod float_nan_misc_edge_semantics;
#[path = "float-operations-comprehensive.rs"]
mod float_operations_comprehensive;
#[path = "following-char-operations.rs"]
mod following_char_operations;
#[path = "font-otf-availability-semantics.rs"]
mod font_otf_availability_semantics;
mod format;
#[path = "format-advanced.rs"]
mod format_advanced;
#[path = "format-comprehensive-patterns.rs"]
mod format_comprehensive_patterns;
#[path = "format-delete-insert-strict-edge-semantics.rs"]
mod format_delete_insert_strict_edge_semantics;
#[path = "format-extended.rs"]
mod format_extended;
#[path = "format-extended-advanced.rs"]
mod format_extended_advanced;
#[path = "format-message-edge-semantics.rs"]
mod format_message_edge_semantics;
#[path = "format-message-patterns.rs"]
mod format_message_patterns;
#[path = "format-patterns.rs"]
mod format_patterns;
#[path = "format-pred-identity-edge-semantics.rs"]
mod format_pred_identity_edge_semantics;
#[path = "format-prompt-semantics.rs"]
mod format_prompt_semantics;
#[path = "format-spec-semantics.rs"]
mod format_spec_semantics;
#[path = "format-string-advanced-patterns.rs"]
mod format_string_advanced_patterns;
#[path = "forward-char.rs"]
mod forward_char;
#[path = "forward-comment.rs"]
mod forward_comment;
#[path = "forward-comment-charset-strict-edge-semantics.rs"]
mod forward_comment_charset_strict_edge_semantics;
#[path = "forward-comment-patterns.rs"]
mod forward_comment_patterns;
#[path = "forward-line.rs"]
mod forward_line;
#[path = "forward-line-advanced.rs"]
mod forward_line_advanced;
#[path = "frame-configuration-semantics.rs"]
mod frame_configuration_semantics;
#[path = "frame-window-strict-edge-semantics.rs"]
mod frame_window_strict_edge_semantics;
#[path = "frame-windows-min-size-semantics.rs"]
mod frame_windows_min_size_semantics;
#[path = "fset-marker-alias-edge-semantics.rs"]
mod fset_marker_alias_edge_semantics;
#[path = "fset-symbol-function.rs"]
mod fset_symbol_function;
#[path = "func-arity-semantics.rs"]
mod func_arity_semantics;
mod funcall;
#[path = "funcall-apply-comprehensive.rs"]
mod funcall_apply_comprehensive;
#[path = "function-cell-semantics.rs"]
mod function_cell_semantics;
#[path = "function-equal-semantics.rs"]
mod function_equal_semantics;
#[path = "function-get-semantics.rs"]
mod function_get_semantics;
#[path = "function-introspect-strict-edge-semantics.rs"]
mod function_introspect_strict_edge_semantics;
#[path = "function-introspection-via-binary-semantics.rs"]
mod function_introspection_via_binary_semantics;
#[path = "gc-scan-strict-edge-semantics.rs"]
mod gc_scan_strict_edge_semantics;
#[path = "generator-semantics.rs"]
mod generator_semantics;
#[path = "generic-function-comprehensive.rs"]
mod generic_function_comprehensive;
mod r#get;
#[path = "get-buffer-semantics.rs"]
mod get_buffer_semantics;
#[path = "get-file-buffer-semantics.rs"]
mod get_file_buffer_semantics;
#[path = "global-local-keymap-semantics.rs"]
mod global_local_keymap_semantics;
#[path = "goto-char.rs"]
mod goto_char;
#[path = "goto-char-advanced.rs"]
mod goto_char_advanced;
#[path = "hash-table.rs"]
mod hash_table;
#[path = "hash-table-advanced.rs"]
mod hash_table_advanced;
#[path = "hash-table-comprehensive-patterns.rs"]
mod hash_table_comprehensive_patterns;
#[path = "hash-table-contains-semantics.rs"]
mod hash_table_contains_semantics;
#[path = "hash-table-deep-edge-semantics.rs"]
mod hash_table_deep_edge_semantics;
#[path = "hash-table-extended.rs"]
mod hash_table_extended;
#[path = "hash-table-mutate-strict-edge-semantics.rs"]
mod hash_table_mutate_strict_edge_semantics;
#[path = "hash-table-operations-comprehensive.rs"]
mod hash_table_operations_comprehensive;
#[path = "hash-table-operations-extended.rs"]
mod hash_table_operations_extended;
#[path = "hash-table-patterns.rs"]
mod hash_table_patterns;
#[path = "hash-table-strict-edge-semantics.rs"]
mod hash_table_strict_edge_semantics;
#[path = "help-doc-semantics.rs"]
mod help_doc_semantics;
#[path = "history-semantics.rs"]
mod history_semantics;
#[path = "hook-mutation-semantics.rs"]
mod hook_mutation_semantics;
#[path = "identity-operations.rs"]
mod identity_operations;
mod r#if;
#[path = "if-advanced.rs"]
mod if_advanced;
#[path = "if-cond-when-unless-comprehensive.rs"]
mod if_cond_when_unless_comprehensive;
#[path = "ignore-error-semantics.rs"]
mod ignore_error_semantics;
#[path = "image-feature-availability-semantics.rs"]
mod image_feature_availability_semantics;
#[path = "increment-compare-pred-strict-edge-semantics.rs"]
mod increment_compare_pred_strict_edge_semantics;
#[path = "indent-to.rs"]
mod indent_to;
#[path = "indirect-function.rs"]
mod indirect_function;
#[path = "inotify-debug-availability-semantics.rs"]
mod inotify_debug_availability_semantics;
#[path = "inotify-public-semantics.rs"]
mod inotify_public_semantics;
mod insert;
#[path = "insert-advanced.rs"]
mod insert_advanced;
#[path = "insert-buffer-comprehensive.rs"]
mod insert_buffer_comprehensive;
#[path = "insert-char-operations.rs"]
mod insert_char_operations;
#[path = "interaction-patterns-strict-edge-semantics.rs"]
mod interaction_patterns_strict_edge_semantics;
#[path = "interactive-form-comprehensive.rs"]
mod interactive_form_comprehensive;
#[path = "interactive-patterns.rs"]
mod interactive_patterns;
#[path = "interactive-patterns-advanced.rs"]
mod interactive_patterns_advanced;
#[path = "intern-concat-strict-edge-semantics.rs"]
mod intern_concat_strict_edge_semantics;
#[path = "intern-soft-advanced.rs"]
mod intern_soft_advanced;
#[path = "internal-event-symbol-advanced.rs"]
mod internal_event_symbol_advanced;
#[path = "invisibility-spec-semantics.rs"]
mod invisibility_spec_semantics;
#[path = "iso8601-semantics.rs"]
mod iso8601_semantics;
#[path = "json-availability-semantics.rs"]
mod json_availability_semantics;
#[path = "json-semantics.rs"]
mod json_semantics;
#[path = "kbd-event-advanced.rs"]
mod kbd_event_advanced;
#[path = "kbd-key-parse-edge-semantics.rs"]
mod kbd_key_parse_edge_semantics;
#[path = "key-description.rs"]
mod key_description;
#[path = "key-subr-meta-strict-edge-semantics.rs"]
mod key_subr_meta_strict_edge_semantics;
#[path = "keyboard-translate-semantics.rs"]
mod keyboard_translate_semantics;
mod keymap;
#[path = "keymap-advanced.rs"]
mod keymap_advanced;
#[path = "keymap-canonicalize-semantics.rs"]
mod keymap_canonicalize_semantics;
#[path = "keymap-comprehensive-patterns.rs"]
mod keymap_comprehensive_patterns;
#[path = "keymap-operations-extended.rs"]
mod keymap_operations_extended;
#[path = "keymap-prompt-patterns.rs"]
mod keymap_prompt_patterns;
#[path = "keymap-strict-edge-semantics.rs"]
mod keymap_strict_edge_semantics;
#[path = "labeled-restriction.rs"]
mod labeled_restriction;
#[path = "lambda-anonymous.rs"]
mod lambda_anonymous;
#[path = "lambda-anonymous-advanced.rs"]
mod lambda_anonymous_advanced;
mod last;
#[path = "lcms-feature-availability-semantics.rs"]
mod lcms_feature_availability_semantics;
#[path = "length-operations.rs"]
mod length_operations;
#[path = "length-semantics.rs"]
mod length_semantics;
mod r#let;
#[path = "let-advanced.rs"]
mod let_advanced;
#[path = "let-binding-comprehensive.rs"]
mod let_binding_comprehensive;
#[path = "let-binding-patterns.rs"]
mod let_binding_patterns;
#[path = "let-dynamic.rs"]
mod let_dynamic;
#[path = "let-lexical-dynamic-patterns.rs"]
mod let_lexical_dynamic_patterns;
#[path = "let-star.rs"]
mod let_star;
#[path = "let-star-advanced.rs"]
mod let_star_advanced;
#[path = "let-star-advanced-2.rs"]
mod let_star_advanced_2;
#[path = "letrec-semantics.rs"]
mod letrec_semantics;
#[path = "lexical-binding-comprehensive.rs"]
mod lexical_binding_comprehensive;
#[path = "lexical-vs-dynamic-comprehensive.rs"]
mod lexical_vs_dynamic_comprehensive;
#[path = "line-edit-helper-semantics.rs"]
mod line_edit_helper_semantics;
#[path = "line-number-misc-strict-edge-semantics.rs"]
mod line_number_misc_strict_edge_semantics;
#[path = "line-position-advanced.rs"]
mod line_position_advanced;
#[path = "lisp-adv-constructs-via-binary-semantics.rs"]
mod lisp_adv_constructs_via_binary_semantics;
#[path = "lisp-constructs-via-binary-semantics.rs"]
mod lisp_constructs_via_binary_semantics;
mod list;
#[path = "list-creation-comprehensive.rs"]
mod list_creation_comprehensive;
#[path = "list-manipulation-comprehensive.rs"]
mod list_manipulation_comprehensive;
#[path = "list-operations-advanced.rs"]
mod list_operations_advanced;
#[path = "list-seq-deep-interaction-semantics.rs"]
mod list_seq_deep_interaction_semantics;
#[path = "list-seq-strict-edge-semantics.rs"]
mod list_seq_strict_edge_semantics;
#[path = "list-tail-helper-semantics.rs"]
mod list_tail_helper_semantics;
#[path = "listify-key-sequence-semantics.rs"]
mod listify_key_sequence_semantics;
#[path = "load-history-semantics.rs"]
mod load_history_semantics;
#[path = "load-suffixes-semantics.rs"]
mod load_suffixes_semantics;
#[path = "locate-dominating-file-strict-edge-semantics.rs"]
mod locate_dominating_file_strict_edge_semantics;
#[path = "locate-file-strict-edge-semantics.rs"]
mod locate_file_strict_edge_semantics;
#[path = "log10-semantics.rs"]
mod log10_semantics;
#[path = "looking-at-advanced.rs"]
mod looking_at_advanced;
#[path = "looking-at-pos-strict-edge-semantics.rs"]
mod looking_at_pos_strict_edge_semantics;
#[path = "lookup-key-advanced.rs"]
mod lookup_key_advanced;
#[path = "lsh-semantics.rs"]
mod lsh_semantics;
#[path = "macro-comprehensive-patterns.rs"]
mod macro_comprehensive_patterns;
#[path = "macroexpand-advanced.rs"]
mod macroexpand_advanced;
#[path = "macrop-obarrayp-daemonp-semantics.rs"]
mod macrop_obarrayp_daemonp_semantics;
#[path = "macrop-via-binary-semantics.rs"]
mod macrop_via_binary_semantics;
#[path = "make-composed-keymap-semantics.rs"]
mod make_composed_keymap_semantics;
#[path = "make-empty-file-strict-edge-semantics.rs"]
mod make_empty_file_strict_edge_semantics;
#[path = "make-hash-table-advanced.rs"]
mod make_hash_table_advanced;
#[path = "make-list.rs"]
mod make_list;
#[path = "make-string.rs"]
mod make_string;
#[path = "make-string-advanced.rs"]
mod make_string_advanced;
#[path = "make-string-patterns.rs"]
mod make_string_patterns;
#[path = "make-symbol.rs"]
mod make_symbol;
#[path = "make-temp-file-strict-edge-semantics.rs"]
mod make_temp_file_strict_edge_semantics;
#[path = "make-vector-advanced.rs"]
mod make_vector_advanced;
#[path = "make-vector-patterns.rs"]
mod make_vector_patterns;
#[path = "map-dedup-tree-strict-edge-semantics.rs"]
mod map_dedup_tree_strict_edge_semantics;
#[path = "map-keymap-sorted-semantics.rs"]
mod map_keymap_sorted_semantics;
#[path = "map-library-semantics.rs"]
mod map_library_semantics;
#[path = "map-operations.rs"]
mod map_operations;
#[path = "map-operations-advanced.rs"]
mod map_operations_advanced;
#[path = "map-semantics.rs"]
mod map_semantics;
#[path = "mapatoms-obarray-comprehensive.rs"]
mod mapatoms_obarray_comprehensive;
#[path = "mapc-operations.rs"]
mod mapc_operations;
mod mapcar;
#[path = "mapcar-mapc-comprehensive.rs"]
mod mapcar_mapc_comprehensive;
#[path = "mapconcat-advanced.rs"]
mod mapconcat_advanced;
#[path = "mapconcat-patterns.rs"]
mod mapconcat_patterns;
#[path = "maphash-patterns.rs"]
mod maphash_patterns;
#[path = "marker-comprehensive-patterns.rs"]
mod marker_comprehensive_patterns;
#[path = "marker-edge-semantics.rs"]
mod marker_edge_semantics;
#[path = "marker-event-strict-edge-semantics.rs"]
mod marker_event_strict_edge_semantics;
#[path = "marker-operations.rs"]
mod marker_operations;
#[path = "marker-region-strict-edge-semantics.rs"]
mod marker_region_strict_edge_semantics;
#[path = "marker-semantics.rs"]
mod marker_semantics;
#[path = "match-beginning.rs"]
mod match_beginning;
#[path = "match-data.rs"]
mod match_data;
#[path = "match-data-advanced.rs"]
mod match_data_advanced;
#[path = "match-data-subexpr-strict-edge-semantics.rs"]
mod match_data_subexpr_strict_edge_semantics;
#[path = "match-end.rs"]
mod match_end;
#[path = "match-string-advanced.rs"]
mod match_string_advanced;
#[path = "match-substitute-replacement-semantics.rs"]
mod match_substitute_replacement_semantics;
#[path = "matching-paren-advanced.rs"]
mod matching_paren_advanced;
#[path = "math-deep-edge-semantics.rs"]
mod math_deep_edge_semantics;
#[path = "math-equal-strict-edge-semantics.rs"]
mod math_equal_strict_edge_semantics;
#[path = "math-functions.rs"]
mod math_functions;
mod max;
#[path = "max-char-operations.rs"]
mod max_char_operations;
#[path = "md5-semantics.rs"]
mod md5_semantics;
mod member;
#[path = "member-alist-semantics.rs"]
mod member_alist_semantics;
mod memq;
#[path = "merge-ordered-lists-semantics.rs"]
mod merge_ordered_lists_semantics;
#[path = "message-format-advanced.rs"]
mod message_format_advanced;
mod min;
#[path = "misc-core-strict-edge-semantics.rs"]
mod misc_core_strict_edge_semantics;
#[path = "misc-fill-strict-edge-semantics.rs"]
mod misc_fill_strict_edge_semantics;
#[path = "missing-subrs-batch-semantics.rs"]
mod missing_subrs_batch_semantics;
#[path = "modify-syntax-entry.rs"]
mod modify_syntax_entry;
#[path = "move-to-column-advanced.rs"]
mod move_to_column_advanced;
#[path = "move-to-column-patterns.rs"]
mod move_to_column_patterns;
#[path = "multibyte-string-comprehensive.rs"]
mod multibyte_string_comprehensive;
#[path = "mutex-lockfile-strict-edge-semantics.rs"]
mod mutex_lockfile_strict_edge_semantics;
#[path = "narrow-advanced.rs"]
mod narrow_advanced;
#[path = "narrow-point-semantics.rs"]
mod narrow_point_semantics;
#[path = "narrow-textprop-strict-edge-semantics.rs"]
mod narrow_textprop_strict_edge_semantics;
#[path = "narrow-widen-comprehensive.rs"]
mod narrow_widen_comprehensive;
#[path = "narrow-widen-patterns.rs"]
mod narrow_widen_patterns;
#[path = "native-comp-available-semantics.rs"]
mod native_comp_available_semantics;
#[path = "nbutlast-butlast-advanced.rs"]
mod nbutlast_butlast_advanced;
#[path = "nbutlast-butlast-semantics.rs"]
mod nbutlast_butlast_semantics;
mod nconc;
#[path = "nconc-advanced.rs"]
mod nconc_advanced;
#[path = "nconc-nreverse-comprehensive.rs"]
mod nconc_nreverse_comprehensive;
#[path = "nconc-nreverse-patterns.rs"]
mod nconc_nreverse_patterns;
#[path = "next-line-goal-column.rs"]
mod next_line_goal_column;
#[path = "next-property-change-advanced.rs"]
mod next_property_change_advanced;
#[path = "next-property-change-patterns.rs"]
mod next_property_change_patterns;
mod r#not;
mod nreverse;
#[path = "nreverse-reverse-semantics.rs"]
mod nreverse_reverse_semantics;
mod nthcdr;
#[path = "nthcdr-advanced.rs"]
mod nthcdr_advanced;
#[path = "nthcdr-last-semantics.rs"]
mod nthcdr_last_semantics;
#[path = "number-arithmetic-comprehensive.rs"]
mod number_arithmetic_comprehensive;
#[path = "number-compare-convert-deep-edge-semantics.rs"]
mod number_compare_convert_deep_edge_semantics;
#[path = "number-conversion-comprehensive.rs"]
mod number_conversion_comprehensive;
#[path = "number-conversion-strict-edge-semantics.rs"]
mod number_conversion_strict_edge_semantics;
#[path = "number-operations-comprehensive.rs"]
mod number_operations_comprehensive;
#[path = "number-predicate-edge-semantics.rs"]
mod number_predicate_edge_semantics;
#[path = "number-predicates.rs"]
mod number_predicates;
#[path = "number-predicates-advanced.rs"]
mod number_predicates_advanced;
#[path = "number-seq-concat-strict-edge-semantics.rs"]
mod number_seq_concat_strict_edge_semantics;
#[path = "number-sequence-advanced.rs"]
mod number_sequence_advanced;
#[path = "number-sequence-operations.rs"]
mod number_sequence_operations;
#[path = "number-to-string.rs"]
mod number_to_string;
#[path = "number-to-string-advanced.rs"]
mod number_to_string_advanced;
#[path = "obarray-buckets-semantics.rs"]
mod obarray_buckets_semantics;
#[path = "obarray-comprehensive-patterns.rs"]
mod obarray_comprehensive_patterns;
#[path = "obarray-patterns.rs"]
mod obarray_patterns;
#[path = "obarray-strict-edge-semantics.rs"]
mod obarray_strict_edge_semantics;
#[path = "obarray-symbol-interning.rs"]
mod obarray_symbol_interning;
#[path = "object-intervals-semantics.rs"]
mod object_intervals_semantics;
mod oclosure;
#[path = "oclosure-advanced.rs"]
mod oclosure_advanced;
mod r#or;
#[path = "overlay-comprehensive-patterns.rs"]
mod overlay_comprehensive_patterns;
#[path = "overlay-helper-semantics.rs"]
mod overlay_helper_semantics;
#[path = "parse-colon-path-strict-edge-semantics.rs"]
mod parse_colon_path_strict_edge_semantics;
#[path = "parse-time-semantics.rs"]
mod parse_time_semantics;
#[path = "pcase-comprehensive-patterns.rs"]
mod pcase_comprehensive_patterns;
mod plist;
#[path = "plist-advanced.rs"]
mod plist_advanced;
#[path = "plist-comprehensive-patterns.rs"]
mod plist_comprehensive_patterns;
#[path = "plist-member-advanced.rs"]
mod plist_member_advanced;
#[path = "plist-obarray-strict-edge-semantics.rs"]
mod plist_obarray_strict_edge_semantics;
#[path = "plist-semantics.rs"]
mod plist_semantics;
mod point;
#[path = "point-max.rs"]
mod point_max;
#[path = "point-min.rs"]
mod point_min;
#[path = "pos-bol-eol-semantics.rs"]
mod pos_bol_eol_semantics;
#[path = "pos-read-byte-strict-edge-semantics.rs"]
mod pos_read_byte_strict_edge_semantics;
#[path = "posix-ntake-keymap-final-strict-edge-semantics.rs"]
mod posix_ntake_keymap_final_strict_edge_semantics;
#[path = "posn-object-semantics.rs"]
mod posn_object_semantics;
#[path = "pp-semantics.rs"]
mod pp_semantics;
#[path = "predicate-logic-comprehensive.rs"]
mod predicate_logic_comprehensive;
mod predicates;
#[path = "primitive-function-p-semantics.rs"]
mod primitive_function_p_semantics;
#[path = "primitive-predicate-edge-semantics.rs"]
mod primitive_predicate_edge_semantics;
#[path = "prin1-comprehensive-patterns.rs"]
mod prin1_comprehensive_patterns;
#[path = "prin1-to-string-advanced.rs"]
mod prin1_to_string_advanced;
#[path = "print-eval-strict-edge-semantics.rs"]
mod print_eval_strict_edge_semantics;
#[path = "process-environment-semantics.rs"]
mod process_environment_semantics;
#[path = "process-lines-semantics.rs"]
mod process_lines_semantics;
#[path = "process-property-semantics.rs"]
mod process_property_semantics;
#[path = "process-string-comprehensive.rs"]
mod process_string_comprehensive;
#[path = "process-thread-mutex-overlay-predicates-semantics.rs"]
mod process_thread_mutex_overlay_predicates_semantics;
#[path = "profiler-memory-semantics.rs"]
mod profiler_memory_semantics;
mod prog1;
#[path = "prog1-prog2-advanced.rs"]
mod prog1_prog2_advanced;
mod progn;
#[path = "progn-advanced.rs"]
mod progn_advanced;
mod progn_ast;
#[path = "progn-prog1-prog2-comprehensive.rs"]
mod progn_prog1_prog2_comprehensive;
#[path = "progress-reporter-semantics.rs"]
mod progress_reporter_semantics;
#[path = "proper-list-predicates.rs"]
mod proper_list_predicates;
#[path = "propertize-advanced.rs"]
mod propertize_advanced;
#[path = "propertize-func-narrow-edge-semantics.rs"]
mod propertize_func_narrow_edge_semantics;
#[path = "property-list-advanced.rs"]
mod property_list_advanced;
#[path = "property-list-comprehensive.rs"]
mod property_list_comprehensive;
#[path = "provide-require-comprehensive.rs"]
mod provide_require_comprehensive;
#[path = "purecopy-strict-edge-semantics.rs"]
mod purecopy_strict_edge_semantics;
mod put;
#[path = "put-text-property-patterns.rs"]
mod put_text_property_patterns;
#[path = "random-garbage-collect-semantics.rs"]
mod random_garbage_collect_semantics;
#[path = "random-operations-comprehensive.rs"]
mod random_operations_comprehensive;
#[path = "random-seed-strict-edge-semantics.rs"]
mod random_seed_strict_edge_semantics;
#[path = "re-search-backward-advanced.rs"]
mod re_search_backward_advanced;
#[path = "re-search-forward.rs"]
mod re_search_forward;
#[path = "re-search-patterns.rs"]
mod re_search_patterns;
#[path = "read-char-semantics.rs"]
mod read_char_semantics;
#[path = "read-from-string-advanced.rs"]
mod read_from_string_advanced;
#[path = "read-from-string-patterns.rs"]
mod read_from_string_patterns;
#[path = "read-from-string-semantics.rs"]
mod read_from_string_semantics;
#[path = "read-print.rs"]
mod read_print;
#[path = "read-print-advanced.rs"]
mod read_print_advanced;
#[path = "read-print-comprehensive.rs"]
mod read_print_comprehensive;
#[path = "readable-function-alias-semantics.rs"]
mod readable_function_alias_semantics;
#[path = "rectangle-semantics.rs"]
mod rectangle_semantics;
mod recursion;
#[path = "recursion-advanced.rs"]
mod recursion_advanced;
#[path = "recursion-comprehensive-patterns.rs"]
mod recursion_comprehensive_patterns;
#[path = "redirect-debugging-output-semantics.rs"]
mod redirect_debugging_output_semantics;
#[path = "regex-macroexpand-via-binary-semantics.rs"]
mod regex_macroexpand_via_binary_semantics;
#[path = "regexp-advanced.rs"]
mod regexp_advanced;
#[path = "regexp-comprehensive-advanced.rs"]
mod regexp_comprehensive_advanced;
#[path = "regexp-comprehensive-patterns.rs"]
mod regexp_comprehensive_patterns;
#[path = "regexp-context-semantics.rs"]
mod regexp_context_semantics;
#[path = "regexp-gnu-divergence.rs"]
mod regexp_gnu_divergence;
#[path = "regexp-match-strict-edge-semantics.rs"]
mod regexp_match_strict_edge_semantics;
#[path = "regexp-operations.rs"]
mod regexp_operations;
#[path = "regexp-operations-advanced.rs"]
mod regexp_operations_advanced;
#[path = "regexp-opt-semantics.rs"]
mod regexp_opt_semantics;
#[path = "regexp-quote-advanced.rs"]
mod regexp_quote_advanced;
#[path = "regexp-quote-patterns.rs"]
mod regexp_quote_patterns;
#[path = "regexp-replace-advanced.rs"]
mod regexp_replace_advanced;
#[path = "regexp-replace-comprehensive.rs"]
mod regexp_replace_comprehensive;
#[path = "regexp-replace-match-strict-edge-semantics.rs"]
mod regexp_replace_match_strict_edge_semantics;
#[path = "region-mark-semantics.rs"]
mod region_mark_semantics;
#[path = "register-semantics.rs"]
mod register_semantics;
#[path = "remember-mouse-glyph-semantics.rs"]
mod remember_mouse_glyph_semantics;
#[path = "remove-text-properties-patterns.rs"]
mod remove_text_properties_patterns;
#[path = "rename-buffer-patterns.rs"]
mod rename_buffer_patterns;
#[path = "rename-file-strict-edge-semantics.rs"]
mod rename_file_strict_edge_semantics;
#[path = "replace-in-region-semantics.rs"]
mod replace_in_region_semantics;
#[path = "replace-match-advanced.rs"]
mod replace_match_advanced;
#[path = "replace-match-patterns.rs"]
mod replace_match_patterns;
#[path = "replace-regexp-advanced.rs"]
mod replace_regexp_advanced;
mod reverse;
#[path = "ring-buffer-comprehensive.rs"]
mod ring_buffer_comprehensive;
#[path = "ring-semantics.rs"]
mod ring_semantics;
#[path = "run-hook-with-args-semantics.rs"]
mod run_hook_with_args_semantics;
#[path = "run-hook-wrapped-semantics.rs"]
mod run_hook_wrapped_semantics;
#[path = "run-hooks-semantics.rs"]
mod run_hooks_semantics;
#[path = "run-mode-hooks-semantics.rs"]
mod run_mode_hooks_semantics;
#[path = "rx-semantics.rs"]
mod rx_semantics;
#[path = "safe-length-operations.rs"]
mod safe_length_operations;
#[path = "safe-length-patterns.rs"]
mod safe_length_patterns;
#[path = "save-excursion.rs"]
mod save_excursion;
#[path = "save-excursion-advanced.rs"]
mod save_excursion_advanced;
#[path = "save-excursion-comprehensive.rs"]
mod save_excursion_comprehensive;
#[path = "save-excursion-patterns.rs"]
mod save_excursion_patterns;
#[path = "save-excursion-restriction-strict-edge-semantics.rs"]
mod save_excursion_restriction_strict_edge_semantics;
#[path = "save-mark-and-excursion-semantics.rs"]
mod save_mark_and_excursion_semantics;
#[path = "save-restriction-advanced.rs"]
mod save_restriction_advanced;
#[path = "save-restriction-comprehensive.rs"]
mod save_restriction_comprehensive;
#[path = "search-backward-advanced.rs"]
mod search_backward_advanced;
#[path = "search-match-deep-via-binary-semantics.rs"]
mod search_match_deep_via_binary_semantics;
#[path = "search-operations.rs"]
mod search_operations;
#[path = "secure-hash-semantics.rs"]
mod secure_hash_semantics;
#[path = "seq-comprehensive-patterns.rs"]
mod seq_comprehensive_patterns;
#[path = "seq-library-comprehensive.rs"]
mod seq_library_comprehensive;
#[path = "seq-operations-advanced.rs"]
mod seq_operations_advanced;
#[path = "seq-operations-comprehensive.rs"]
mod seq_operations_comprehensive;
#[path = "seq-operations-extended.rs"]
mod seq_operations_extended;
#[path = "sequence-access-semantics.rs"]
mod sequence_access_semantics;
#[path = "sequence-edge-semantics.rs"]
mod sequence_edge_semantics;
#[path = "sequence-operations.rs"]
mod sequence_operations;
#[path = "sequence-sorting-comprehensive.rs"]
mod sequence_sorting_comprehensive;
mod sequencep;
#[path = "set-buffer.rs"]
mod set_buffer;
#[path = "set-match-data-patterns.rs"]
mod set_match_data_patterns;
mod setcar;
#[path = "setcar-setcdr-advanced.rs"]
mod setcar_setcdr_advanced;
mod setcdr;
mod setq;
#[path = "setq-advanced.rs"]
mod setq_advanced;
#[path = "setq-setf-comprehensive.rs"]
mod setq_setf_comprehensive;
#[path = "shell-command-to-string-semantics.rs"]
mod shell_command_to_string_semantics;
#[path = "shell-process-command-semantics.rs"]
mod shell_process_command_semantics;
#[path = "shell-quote-semantics.rs"]
mod shell_quote_semantics;
mod signal;
#[path = "signal-advanced.rs"]
mod signal_advanced;
#[path = "signal-error-strict-edge-semantics.rs"]
mod signal_error_strict_edge_semantics;
#[path = "signal-throw-patterns.rs"]
mod signal_throw_patterns;
#[path = "single-key-description-advanced.rs"]
mod single_key_description_advanced;
#[path = "skip-chars.rs"]
mod skip_chars;
#[path = "skip-chars-advanced.rs"]
mod skip_chars_advanced;
#[path = "skip-chars-field-strict-edge-semantics.rs"]
mod skip_chars_field_strict_edge_semantics;
#[path = "skip-chars-patterns.rs"]
mod skip_chars_patterns;
#[path = "skip-syntax-advanced.rs"]
mod skip_syntax_advanced;
#[path = "sleep-for-semantics.rs"]
mod sleep_for_semantics;
mod sort;
#[path = "sort-algorithms.rs"]
mod sort_algorithms;
#[path = "sort-command-semantics.rs"]
mod sort_command_semantics;
#[path = "sort-compare-strict-edge-semantics.rs"]
mod sort_compare_strict_edge_semantics;
#[path = "sort-extended.rs"]
mod sort_extended;
#[path = "sort-mapcar-append-strict-edge-semantics.rs"]
mod sort_mapcar_append_strict_edge_semantics;
#[path = "sort-semantics.rs"]
mod sort_semantics;
#[path = "sort-stable-patterns.rs"]
mod sort_stable_patterns;
#[path = "special-form-deep-edge-semantics.rs"]
mod special_form_deep_edge_semantics;
#[path = "special-forms-semantics.rs"]
mod special_forms_semantics;
#[path = "special-forms-strict-edge-semantics.rs"]
mod special_forms_strict_edge_semantics;
#[path = "split-string-advanced.rs"]
mod split_string_advanced;
#[path = "split-string-patterns.rs"]
mod split_string_patterns;
#[path = "sqlite-values-validation-semantics.rs"]
mod sqlite_values_validation_semantics;
mod string;
#[path = "string-builder-comprehensive.rs"]
mod string_builder_comprehensive;
#[path = "string-bytes-width-advanced.rs"]
mod string_bytes_width_advanced;
#[path = "string-compare-deep-edge-semantics.rs"]
mod string_compare_deep_edge_semantics;
#[path = "string-compare-match-strict-edge-semantics.rs"]
mod string_compare_match_strict_edge_semantics;
#[path = "string-comparison-comprehensive.rs"]
mod string_comparison_comprehensive;
#[path = "string-conversion-semantics.rs"]
mod string_conversion_semantics;
#[path = "string-core-semantics.rs"]
mod string_core_semantics;
#[path = "string-distance.rs"]
mod string_distance;
#[path = "string-distance-advanced.rs"]
mod string_distance_advanced;
#[path = "string-distance-patterns.rs"]
mod string_distance_patterns;
#[path = "string-encoding-comprehensive.rs"]
mod string_encoding_comprehensive;
#[path = "string-equal.rs"]
mod string_equal;
#[path = "string-equal-propertize-strict-semantics.rs"]
mod string_equal_propertize_strict_semantics;
#[path = "string-join-patterns.rs"]
mod string_join_patterns;
#[path = "string-length-value-comparison-semantics.rs"]
mod string_length_value_comparison_semantics;
#[path = "string-lessp.rs"]
mod string_lessp;
#[path = "string-lines-semantics.rs"]
mod string_lines_semantics;
#[path = "string-list-strict-edge-semantics.rs"]
mod string_list_strict_edge_semantics;
#[path = "string-manipulation.rs"]
mod string_manipulation;
#[path = "string-manipulation-advanced.rs"]
mod string_manipulation_advanced;
#[path = "string-manipulation-comprehensive.rs"]
mod string_manipulation_comprehensive;
#[path = "string-match.rs"]
mod string_match;
#[path = "string-match-p.rs"]
mod string_match_p;
#[path = "string-match-p-semantics.rs"]
mod string_match_p_semantics;
#[path = "string-number-byte-deep-edge-semantics.rs"]
mod string_number_byte_deep_edge_semantics;
#[path = "string-number-list-deep-edge-semantics.rs"]
mod string_number_list_deep_edge_semantics;
#[path = "string-prefix-suffix-patterns.rs"]
mod string_prefix_suffix_patterns;
#[path = "string-processing.rs"]
mod string_processing;
#[path = "string-processing-advanced.rs"]
mod string_processing_advanced;
#[path = "string-replace.rs"]
mod string_replace;
#[path = "string-replace-patterns.rs"]
mod string_replace_patterns;
#[path = "string-replace-semantics.rs"]
mod string_replace_semantics;
#[path = "string-search-advanced.rs"]
mod string_search_advanced;
#[path = "string-to-char-advanced.rs"]
mod string_to_char_advanced;
#[path = "string-to-number.rs"]
mod string_to_number;
#[path = "string-to-number-advanced.rs"]
mod string_to_number_advanced;
#[path = "string-to-number-comprehensive.rs"]
mod string_to_number_comprehensive;
#[path = "string-to-number-edge-semantics.rs"]
mod string_to_number_edge_semantics;
#[path = "string-trim-patterns.rs"]
mod string_trim_patterns;
#[path = "string-version-lessp.rs"]
mod string_version_lessp;
#[path = "string-version-lessp-advanced.rs"]
mod string_version_lessp_advanced;
#[path = "string-width-advanced.rs"]
mod string_width_advanced;
#[path = "subr-arity-advanced.rs"]
mod subr_arity_advanced;
#[path = "subr-arity-patterns.rs"]
mod subr_arity_patterns;
#[path = "subr-basic-macro-semantics.rs"]
mod subr_basic_macro_semantics;
#[path = "subr-misc-strict-edge-semantics.rs"]
mod subr_misc_strict_edge_semantics;
#[path = "subr-motion-semantics.rs"]
mod subr_motion_semantics;
#[path = "subr-operations-comprehensive.rs"]
mod subr_operations_comprehensive;
#[path = "subr-predicates.rs"]
mod subr_predicates;
#[path = "subr-x-comprehensive.rs"]
mod subr_x_comprehensive;
#[path = "subst-char-in-string-comprehensive.rs"]
mod subst_char_in_string_comprehensive;
#[path = "subst-char-in-string-semantics.rs"]
mod subst_char_in_string_semantics;
#[path = "substitute-in-file-name-strict-edge-semantics.rs"]
mod substitute_in_file_name_strict_edge_semantics;
#[path = "substitute-key-definition-semantics.rs"]
mod substitute_key_definition_semantics;
mod substring;
#[path = "substring-advanced.rs"]
mod substring_advanced;
#[path = "substring-strict-edge-semantics.rs"]
mod substring_strict_edge_semantics;
#[path = "suppress-keymap-semantics.rs"]
mod suppress_keymap_semantics;
#[path = "surface-diverges.rs"]
mod surface_diverges;
mod symbol;
#[path = "symbol-accessor-strict-edge-semantics.rs"]
mod symbol_accessor_strict_edge_semantics;
#[path = "symbol-advanced.rs"]
mod symbol_advanced;
#[path = "symbol-comprehensive-patterns.rs"]
mod symbol_comprehensive_patterns;
#[path = "symbol-file-semantics.rs"]
mod symbol_file_semantics;
#[path = "symbol-obarray-intern-interaction-semantics.rs"]
mod symbol_obarray_intern_interaction_semantics;
#[path = "symbol-plist-edge-semantics.rs"]
mod symbol_plist_edge_semantics;
#[path = "symbol-plist-identity-semantics.rs"]
mod symbol_plist_identity_semantics;
#[path = "symbol-plist-patterns.rs"]
mod symbol_plist_patterns;
#[path = "symbol-properties-advanced.rs"]
mod symbol_properties_advanced;
#[path = "symbol-property-semantics.rs"]
mod symbol_property_semantics;
#[path = "symbol-value-edge-semantics.rs"]
mod symbol_value_edge_semantics;
#[path = "symbol-with-pos-semantics.rs"]
mod symbol_with_pos_semantics;
#[path = "syntax-local-strict-edge-semantics.rs"]
mod syntax_local_strict_edge_semantics;
#[path = "syntax-parse-state.rs"]
mod syntax_parse_state;
#[path = "syntax-table.rs"]
mod syntax_table;
#[path = "syntax-table-advanced.rs"]
mod syntax_table_advanced;
#[path = "syntax-table-comprehensive.rs"]
mod syntax_table_comprehensive;
#[path = "syntax-table-operations.rs"]
mod syntax_table_operations;
mod take;
#[path = "take-drop-while-semantics.rs"]
mod take_drop_while_semantics;
#[path = "take-ntake-semantics.rs"]
mod take_ntake_semantics;
#[path = "temporary-file-directory-strict-edge-semantics.rs"]
mod temporary_file_directory_strict_edge_semantics;
#[path = "text-prop-search-strict-edge-semantics.rs"]
mod text_prop_search_strict_edge_semantics;
#[path = "text-properties.rs"]
mod text_properties;
#[path = "text-properties-advanced.rs"]
mod text_properties_advanced;
#[path = "text-properties-comprehensive.rs"]
mod text_properties_comprehensive;
#[path = "text-properties-patterns.rs"]
mod text_properties_patterns;
#[path = "text-property-api-semantics.rs"]
mod text_property_api_semantics;
#[path = "text-property-boundary-strict-edge-semantics.rs"]
mod text_property_boundary_strict_edge_semantics;
#[path = "text-property-comprehensive.rs"]
mod text_property_comprehensive;
#[path = "text-property-manipulation.rs"]
mod text_property_manipulation;
#[path = "text-property-order-semantics.rs"]
mod text_property_order_semantics;
#[path = "text-property-search.rs"]
mod text_property_search;
#[path = "thing-at-point-api-semantics.rs"]
mod thing_at_point_api_semantics;
#[path = "thing-at-point-comprehensive.rs"]
mod thing_at_point_comprehensive;
mod r#throw;
#[path = "throw-unwind-strict-edge-semantics.rs"]
mod throw_unwind_strict_edge_semantics;
#[path = "time-date-semantics.rs"]
mod time_date_semantics;
#[path = "timer-list-comprehensive.rs"]
mod timer_list_comprehensive;
#[path = "transpose-command-semantics.rs"]
mod transpose_command_semantics;
mod trigonometry;
#[path = "trigonometry-advanced.rs"]
mod trigonometry_advanced;
#[path = "truncate-string-to-width-semantics.rs"]
mod truncate_string_to_width_semantics;
#[path = "tty-display-dimensions-semantics.rs"]
mod tty_display_dimensions_semantics;
#[path = "tty-frame-at-semantics.rs"]
mod tty_frame_at_semantics;
#[path = "type-of.rs"]
mod type_of;
#[path = "type-of-advanced.rs"]
mod type_of_advanced;
#[path = "type-of-patterns.rs"]
mod type_of_patterns;
#[path = "type-pred-strict-edge-semantics.rs"]
mod type_pred_strict_edge_semantics;
#[path = "type-predicates.rs"]
mod type_predicates;
#[path = "type-predicates-advanced.rs"]
mod type_predicates_advanced;
#[path = "type-predicates-comprehensive.rs"]
mod type_predicates_comprehensive;
#[path = "type-system-strict-edge-semantics.rs"]
mod type_system_strict_edge_semantics;
#[path = "undo-core-semantics.rs"]
mod undo_core_semantics;
mod unless;
#[path = "unwind-protect.rs"]
mod unwind_protect;
#[path = "unwind-protect-advanced.rs"]
mod unwind_protect_advanced;
#[path = "unwind-protect-comprehensive.rs"]
mod unwind_protect_comprehensive;
#[path = "upcase-downcase.rs"]
mod upcase_downcase;
#[path = "upcase-downcase-advanced.rs"]
mod upcase_downcase_advanced;
#[path = "upcase-downcase-patterns.rs"]
mod upcase_downcase_patterns;
#[path = "upcase-initials-patterns.rs"]
mod upcase_initials_patterns;
#[path = "url-parse-semantics.rs"]
mod url_parse_semantics;
#[path = "url-util-file-semantics.rs"]
mod url_util_file_semantics;
#[path = "url-util-semantics.rs"]
mod url_util_semantics;
#[path = "use-map-buffer-swap-strict-edge-semantics.rs"]
mod use_map_buffer_swap_strict_edge_semantics;
#[path = "value-order-semantics.rs"]
mod value_order_semantics;
#[path = "values-store-semantics.rs"]
mod values_store_semantics;
#[path = "variable-alias-semantics.rs"]
mod variable_alias_semantics;
#[path = "variable-watcher-semantics.rs"]
mod variable_watcher_semantics;
#[path = "vconcat-advanced.rs"]
mod vconcat_advanced;
#[path = "vconcat-operations.rs"]
mod vconcat_operations;
mod vector;
#[path = "vector-advanced.rs"]
mod vector_advanced;
#[path = "vector-comprehensive-patterns.rs"]
mod vector_comprehensive_patterns;
#[path = "vector-operations.rs"]
mod vector_operations;
#[path = "vector-operations-comprehensive.rs"]
mod vector_operations_comprehensive;
#[path = "vector-or-char-table-operations.rs"]
mod vector_or_char_table_operations;
#[path = "vector-plist-gensym-read-edge-semantics.rs"]
mod vector_plist_gensym_read_edge_semantics;
#[path = "version-semantics.rs"]
mod version_semantics;
#[path = "warning-macro-semantics.rs"]
mod warning_macro_semantics;
mod when;
#[path = "when-unless-comprehensive.rs"]
mod when_unless_comprehensive;
mod r#while;
#[path = "while-advanced.rs"]
mod while_advanced;
#[path = "while-dolist-dotimes-patterns.rs"]
mod while_dolist_dotimes_patterns;
#[path = "while-loop-advanced-patterns.rs"]
mod while_loop_advanced_patterns;
#[path = "while-loop-comprehensive.rs"]
mod while_loop_comprehensive;
#[path = "while-loop-patterns.rs"]
mod while_loop_patterns;
#[path = "while-patterns.rs"]
mod while_patterns;
#[path = "window-operations-comprehensive.rs"]
mod window_operations_comprehensive;
#[path = "window-tree-primitive-semantics.rs"]
mod window_tree_primitive_semantics;
#[path = "windowp-framep-semantics.rs"]
mod windowp_framep_semantics;
#[path = "with-current-buffer-comprehensive.rs"]
mod with_current_buffer_comprehensive;
#[path = "with-current-buffer-patterns.rs"]
mod with_current_buffer_patterns;
#[path = "with-output-to-string-semantics.rs"]
mod with_output_to_string_semantics;
#[path = "with-output-to-temp-buffer.rs"]
mod with_output_to_temp_buffer;
#[path = "with-temp-buffer.rs"]
mod with_temp_buffer;
#[path = "with-temp-buffer-advanced-patterns.rs"]
mod with_temp_buffer_advanced_patterns;
#[path = "with-temp-buffer-comprehensive.rs"]
mod with_temp_buffer_comprehensive;
#[path = "with-temp-file-semantics.rs"]
mod with_temp_file_semantics;
#[path = "wrapper-hook-semantics.rs"]
mod wrapper_hook_semantics;
#[path = "xml-semantics.rs"]
mod xml_semantics;
#[path = "yank-properties-semantics.rs"]
mod yank_properties_semantics;
#[path = "zlib-decompress-region-semantics.rs"]
mod zlib_decompress_region_semantics;

#[path = "divergence-advice-deep.rs"]
mod divergence_advice_deep;
#[path = "divergence-advice-hooks-locals.rs"]
mod divergence_advice_hooks_locals;
#[path = "divergence-arithmetic-float.rs"]
mod divergence_arithmetic_float;
#[path = "divergence-bignum-fixnum-deep.rs"]
mod divergence_bignum_fixnum_deep;
#[path = "divergence-buffer-editing.rs"]
mod divergence_buffer_editing;
#[path = "divergence-buffer-local-killring-fileio.rs"]
mod divergence_buffer_local_killring_fileio;
#[path = "divergence-buffer-locals-hooks.rs"]
mod divergence_buffer_locals_hooks;
#[path = "divergence-buffer-management.rs"]
mod divergence_buffer_management;
#[path = "divergence-buffer-manip-rect.rs"]
mod divergence_buffer_manip_rect;
#[path = "divergence-buffer-motion-search.rs"]
mod divergence_buffer_motion_search;
#[path = "divergence-calendar-time-deep.rs"]
mod divergence_calendar_time_deep;
#[path = "divergence-chartab-charset-deep.rs"]
mod divergence_chartab_charset_deep;
#[path = "divergence-chartab-syntax-deep.rs"]
mod divergence_chartab_syntax_deep;
#[path = "divergence-cllib-deep.rs"]
mod divergence_cllib_deep;
#[path = "divergence-cllib-seq-map.rs"]
mod divergence_cllib_seq_map;
#[path = "divergence-coding-charset-deep.rs"]
mod divergence_coding_charset_deep;
#[path = "divergence-coding-process.rs"]
mod divergence_coding_process;
#[path = "divergence-combo-operations.rs"]
mod divergence_combo_operations;
#[path = "divergence-data-types-deep.rs"]
mod divergence_data_types_deep;
#[path = "divergence-debug-trace-ert.rs"]
mod divergence_debug_trace_ert;
#[path = "divergence-devtools-stubs.rs"]
mod divergence_devtools_stubs;
#[path = "divergence-display-table-glyph.rs"]
mod divergence_display_table_glyph;
#[path = "divergence-eieio-oop.rs"]
mod divergence_eieio_oop;
#[path = "divergence-error-signaling-deep.rs"]
mod divergence_error_signaling_deep;
#[path = "divergence-eval-deep-edge.rs"]
mod divergence_eval_deep_edge;
#[path = "divergence-eval-load-read.rs"]
mod divergence_eval_load_read;
#[path = "divergence-face-custom-theme.rs"]
mod divergence_face_custom_theme;
#[path = "divergence-file-ops-deep.rs"]
mod divergence_file_ops_deep;
#[path = "divergence-file-remote-tramp.rs"]
mod divergence_file_remote_tramp;
#[path = "divergence-fill-abbrev-comment.rs"]
mod divergence_fill_abbrev_comment;
#[path = "divergence-fontlock-jitlock-highlight.rs"]
mod divergence_fontlock_jitlock_highlight;
#[path = "divergence-format-read-integers.rs"]
mod divergence_format_read_integers;
#[path = "divergence-format-string-deep.rs"]
mod divergence_format_string_deep;
#[path = "divergence-frame-display-info.rs"]
mod divergence_frame_display_info;
#[path = "divergence-frame-font-display.rs"]
mod divergence_frame_font_display;
#[path = "divergence-gc-memory-modules.rs"]
mod divergence_gc_memory_modules;
#[path = "divergence-hash-struct-records.rs"]
mod divergence_hash_struct_records;
#[path = "divergence-help-apropos-completion.rs"]
mod divergence_help_apropos_completion;
#[path = "divergence-image-operations.rs"]
mod divergence_image_operations;
#[path = "divergence-image-widget-display.rs"]
mod divergence_image_widget_display;
#[path = "divergence-introspection-version.rs"]
mod divergence_introspection_version;
#[path = "divergence-keyboard-input-methods.rs"]
mod divergence_keyboard_input_methods;
#[path = "divergence-keymap-input-deep.rs"]
mod divergence_keymap_input_deep;
#[path = "divergence-keymap-syntax-category.rs"]
mod divergence_keymap_syntax_category;
#[path = "divergence-kmacro-persistence.rs"]
mod divergence_kmacro_persistence;
#[path = "divergence-lambda-apply-dispatch.rs"]
mod divergence_lambda_apply_dispatch;
#[path = "divergence-load-native-comp.rs"]
mod divergence_load_native_comp;
#[path = "divergence-macro-expansion-stress.rs"]
mod divergence_macro_expansion_stress;
#[path = "divergence-macro-pcase-cllib.rs"]
mod divergence_macro_pcase_cllib;
#[path = "divergence-marker-undo-deep.rs"]
mod divergence_marker_undo_deep;
#[path = "divergence-minibuf-completion-ring.rs"]
mod divergence_minibuf_completion_ring;
#[path = "divergence-misc-builtins.rs"]
mod divergence_misc_builtins;
#[path = "divergence-misc-remaining.rs"]
mod divergence_misc_remaining;
#[path = "divergence-mode-line-misc.rs"]
mod divergence_mode_line_misc;
#[path = "divergence-multibyte-bidi-deep.rs"]
mod divergence_multibyte_bidi_deep;
#[path = "divergence-narrowing-edge.rs"]
mod divergence_narrowing_edge;
#[path = "divergence-narrowing-multibyte.rs"]
mod divergence_narrowing_multibyte;
#[path = "divergence-net-thread-json-xml.rs"]
mod divergence_net_thread_json_xml;
#[path = "divergence-net-xml-dom-mail.rs"]
mod divergence_net_xml_dom_mail;
#[path = "divergence-network-mail-web.rs"]
mod divergence_network_mail_web;
#[path = "divergence-obarray-symbol-deep.rs"]
mod divergence_obarray_symbol_deep;
#[path = "divergence-overlay-deep.rs"]
mod divergence_overlay_deep;
#[path = "divergence-package-dired-elp.rs"]
mod divergence_package_dired_elp;
#[path = "divergence-pcase-deep.rs"]
mod divergence_pcase_deep;
#[path = "divergence-print-circle-read.rs"]
mod divergence_print_circle_read;
#[path = "divergence-print-format-charset.rs"]
mod divergence_print_format_charset;
#[path = "divergence-process-shell.rs"]
mod divergence_process_shell;
#[path = "divergence-process-shell-deep.rs"]
mod divergence_process_shell_deep;
#[path = "divergence-profiling-memory.rs"]
mod divergence_profiling_memory;
#[path = "divergence-project-xref-vcs.rs"]
mod divergence_project_xref_vcs;
#[path = "divergence-quit-error-hierarchy.rs"]
mod divergence_quit_error_hierarchy;
#[path = "divergence-read-symbol-obarray.rs"]
mod divergence_read_symbol_obarray;
#[path = "divergence-reader-printer.rs"]
mod divergence_reader_printer;
#[path = "divergence-regex-deep.rs"]
mod divergence_regex_deep;
#[path = "divergence-regex-string-search.rs"]
mod divergence_regex_string_search;
#[path = "divergence-region-mark-edit.rs"]
mod divergence_region_mark_edit;
#[path = "divergence-register-narrow-misc.rs"]
mod divergence_register_narrow_misc;
#[path = "divergence-rx-pcase-pattern.rs"]
mod divergence_rx_pcase_pattern;
#[path = "divergence-rx-regex-builder.rs"]
mod divergence_rx_regex_builder;
#[path = "divergence-search-charfold-occur.rs"]
mod divergence_search_charfold_occur;
#[path = "divergence-sequence-collections.rs"]
mod divergence_sequence_collections;
#[path = "divergence-sort-type-predicates.rs"]
mod divergence_sort_type_predicates;
#[path = "divergence-stress-combo.rs"]
mod divergence_stress_combo;
#[path = "divergence-stress-combo-2.rs"]
mod divergence_stress_combo_2;
#[path = "divergence-stress-large-deep.rs"]
mod divergence_stress_large_deep;
#[path = "divergence-string-ops.rs"]
mod divergence_string_ops;
#[path = "divergence-subr-bytecode-deep.rs"]
mod divergence_subr_bytecode_deep;
#[path = "divergence-symbol-reader-deep.rs"]
mod divergence_symbol_reader_deep;
#[path = "divergence-terminal-eshell.rs"]
mod divergence_terminal_eshell;
#[path = "divergence-textprop-deep.rs"]
mod divergence_textprop_deep;
#[path = "divergence-textprop-manipulation.rs"]
mod divergence_textprop_manipulation;
#[path = "divergence-textprop-overlay.rs"]
mod divergence_textprop_overlay;
#[path = "divergence-textprop-sticky-deep.rs"]
mod divergence_textprop_sticky_deep;
#[path = "divergence-time-process-final.rs"]
mod divergence_time_process_final;
#[path = "divergence-timer-eventloop.rs"]
mod divergence_timer_eventloop;
#[path = "divergence-treesit-deep.rs"]
mod divergence_treesit_deep;
#[path = "divergence-ui-interaction.rs"]
mod divergence_ui_interaction;
#[path = "divergence-undo-deep-2.rs"]
mod divergence_undo_deep_2;
#[path = "divergence-undo-semantics.rs"]
mod divergence_undo_semantics;
#[path = "divergence-unicode-normalization.rs"]
mod divergence_unicode_normalization;
#[path = "divergence-weak-hash-lifecycle.rs"]
mod divergence_weak_hash_lifecycle;
#[path = "divergence-window-frame-display.rs"]
mod divergence_window_frame_display;
#[path = "divergence-window-geometry.rs"]
mod divergence_window_geometry;
#[path = "divergence-window-redisplay.rs"]
mod divergence_window_redisplay;
