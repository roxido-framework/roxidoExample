#' Zero-Finding
#'
#' This function demonstrates a Rust implementation of the zero-finding
#' algorithm shown in Section 5.11.1 "Zero-finding" of [*Writing R
#' Extensions*](https://cran.r-project.org/doc/manuals/R-exts.html#Zero_002dfinding)
#'
#' @param f A function taking a numeric vector of length one.
#' @param guess1 A guess for the zero of the function.
#' @param guess2 Another guess for the zero of the function. The sign of the function
#'   evaluated at the two guesses must be opposite.
#' @param tol Tolerance controlling the desired precision.
#'
#' @export
#' @examples
#' cube1 <- function(x) (x^2 + 1) * (x - 1.5)
#' zero(cube1, -2, 11.5)
zero <- function(f, guess1, guess2, tol = 1e-7) {
  .Call(.zero, f, guess1, guess2, tol)
}
