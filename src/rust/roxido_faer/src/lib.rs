use faer::MatRef;
use roxido::r::{Matrix, Mutable};
use roxido::*;

pub trait RMatrix2Faer {
    fn as_faer(&self) -> Result<MatRef<'static, f64>, &'static str>;
}

impl RMatrix2Faer for RObject<Matrix, f64> {
    fn as_faer(&self) -> Result<MatRef<'static, f64>, &'static str> {
        Ok({
            let nrow = self.nrow();
            unsafe {
                faer::mat::from_raw_parts(
                    self.slice().as_ptr(),
                    nrow,
                    self.ncol(),
                    1,
                    nrow.try_into().unwrap(),
                )
            }
        })
    }
}

pub trait ToR1<S, T, U> {
    fn to_r(&self, pc: &mut Pc) -> RObject<S, T, U>;
}

impl ToR1<Matrix, f64, Mutable> for MatRef<'_, f64> {
    fn to_r(&self, pc: &mut Pc) -> RObject<Matrix, f64, Mutable> {
        let nr = self.nrows();
        let nc = self.ncols();
        let mut result = R::new_matrix_double(nr, nc, pc);
        for (k, r) in result.slice_mut().iter_mut().enumerate() {
            *r = self.read(k % nr, k / nc);
        }
        result
    }
}
