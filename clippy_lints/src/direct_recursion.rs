use clippy_utils::diagnostics::span_lint;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::LocalDefId;
use rustc_hir::{Expr, ExprKind, HirId, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::TyCtxt;
use rustc_session::declare_lint_pass;
declare_clippy_lint! {
    /// ### What it does
    /// Checks for functions that call themselves from their body.
    ///
    /// ### Why restrict this?
    /// In Safety Critical contexts, recursive calls can lead to catastrophic
    /// crashes if they happen to overflow the stack.
    ///
    /// In most contexts, this is not an issue, so this lint is allow-by-default.
    ///
    /// ### Notes
    ///
    /// #### Triggers
    /// There are two things that trigger this lint:
    /// - Function calls from a function (or method) to itself,
    /// - Function pointer bindings from a function (or method) to itself.
    ///
    /// #### Independent of control flow
    /// This lint triggers whenever the conditions above are met, regardless of
    /// control flow and other such constructs.
    ///
    /// #### Blessing a recursive call
    /// The user may choose to bless a recursive call or binding using the
    /// attribute #<TODO>
    ///
    /// #### Indirect calls
    /// This lint **does not** detect indirect recursive calls.
    ///
    /// ### Examples
    /// This function will trigger the lint:
    /// ```no_run
    /// fn i_call_myself_in_a_bounded_way(bound: u8) {
    ///     if bound > 0 {
    ///         // This line will trigger the lint
    ///         i_call_myself_in_a_bounded_way(bound - 1);
    ///     }
    /// }
    /// ```
    /// Using #<TODO> lets it pass:
    /// ```no_run
    /// fn i_call_myself_in_a_bounded_way(bound: u8) {
    ///     if bound > 0 {
    ///         [#<TODO>]
    ///         i_call_myself_in_a_bounded_way(bound - 1);
    ///     }
    /// }
    /// ```
    /// This triggers the lint when `fibo` is bound to a function pointer
    /// inside `fibo`'s body
    /// ```no_run
    /// fn fibo(a: u32) -> u32 {
    ///     if a < 2 { a } else { (a - 2..a).map(fibo).sum() }
    /// }
    /// ```
    #[clippy::version = "1.89.0"]
    pub DIRECT_RECURSION,
    restriction,
    "functions shall not call themselves directly"
}
declare_lint_pass!(DirectRecursion => [DIRECT_RECURSION]);

impl<'tcx> LateLintPass<'tcx> for DirectRecursion {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if let ExprKind::Path(QPath::Resolved(_, path)) = expr.kind {
            match path.res {
                Res::Def(DefKind::Fn, fn_path_id) | Res::Def(DefKind::AssocFn, fn_path_id) => {
                    let mut origin = expr.hir_id;

                    while let Some(encloser_local_def) = my_enclosing_body_owner(cx.tcx, origin) {
                        let encloser = cx.tcx.local_def_id_to_hir_id(encloser_local_def);
                        if fn_path_id == encloser_local_def.into() {
                            span_lint(
                                cx,
                                DIRECT_RECURSION,
                                expr.span,
                                "this function contains a call to itself",
                            );
                            break;
                        } else {
                            origin = encloser;
                        }
                    }
                },
                _ => {
                    // We won't attempt to reason about it if it's not a direct path to a fn
                    // definition
                },
            }
        }
    }
}

/// Taken from https://doc.rust-lang.org/nightly/nightly-rustc/src/rustc_middle/hir/map.rs.html#236-244
/// I wanted a function like this that did not panic in the "no enclosing owner" case
fn my_enclosing_body_owner<'tcx>(cx: TyCtxt<'tcx>, hir_id: HirId) -> Option<LocalDefId> {
    for (_, node) in cx.hir_parent_iter(hir_id) {
        if let Some((def_id, _)) = node.associated_body() {
            return Some(def_id);
        }
    }
    None
}
