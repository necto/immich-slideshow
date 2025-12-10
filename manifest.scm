;; manifest.scm — Reproducible Rust development environment
;;
;; Usage:
;;    guix shell -m manifest.scm
;;
;; Provides:
;;   • rustc (from rust: default output)
;;   • cargo  (from rust: "cargo" output)
;;   • rustfmt, clippy, etc. (from rust: "tools" output)
;;   • rust-analyzer for IDEs (LSP)
;;   • pkg-config and openssl are optional but commonly needed
;;

(use-modules (gnu)
             (gnu packages)
             (gnu packages rust)
             (gnu packages pkg-config)
             (gnu packages tls))

(packages->manifest
 (list
  rust                  ; rustc, std, documentation, etc.
  (list rust "cargo")   ; cargo binary
  (list rust "tools")   ; rustfmt, clippy, etc.
  rust-analyzer         ; LSP for editors (VSCode, Emacs, Helix, Neovim)

  ;; Optional common dependencies for Rust crates:
  pkg-config
  openssl
  ))
