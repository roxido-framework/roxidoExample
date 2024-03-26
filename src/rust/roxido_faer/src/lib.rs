use faer::MatRef;
use roxido::*;

pub trait RMatrix2Faer {
    fn as_faer(&self) -> Result<MatRef<'static, f64>, &'static str>;
}

impl RMatrix2Faer for RObject<RMatrix, f64> {
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

pub trait ToR<'a, RType, RMode> {
    #[allow(clippy::mut_from_ref)]
    fn to_r(&self, pc: &'a Pc) -> &'a mut RObject<RType, RMode>;
}

impl<'a> ToR<'a, RMatrix, f64> for MatRef<'a, f64> {
    fn to_r(&self, pc: &'a Pc) -> &'a mut RObject<RMatrix, f64> {
        let nr = self.nrows();
        let nc = self.ncols();
        let result = RObject::<RMatrix, f64>::new(nr, nc, pc);
        for (k, r) in result.slice_mut().iter_mut().enumerate() {
            *r = self.read(k % nr, k / nc);
        }
        result
    }
}
