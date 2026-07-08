#[cfg(not(any(
    feature = "default-linalg",
    feature = "intel-mkl",
    feature = "openblas-static",
    feature = "openblas-system",
    feature = "pure-linalg"
)))]
compile_error!(
    "speakrs requires a linear algebra backend; enable default features or choose exactly one of `intel-mkl`, `openblas-static`, `openblas-system`, or `pure-linalg`"
);

#[cfg(any(
    all(
        feature = "default-linalg",
        any(
            feature = "intel-mkl",
            feature = "openblas-static",
            feature = "openblas-system",
            feature = "pure-linalg"
        )
    ),
    all(feature = "intel-mkl", feature = "openblas-static"),
    all(feature = "intel-mkl", feature = "openblas-system"),
    all(feature = "intel-mkl", feature = "pure-linalg"),
    all(feature = "openblas-static", feature = "openblas-system"),
    all(feature = "openblas-static", feature = "pure-linalg"),
    all(feature = "openblas-system", feature = "pure-linalg")
))]
compile_error!(
    "speakrs supports only one linear algebra backend; disable default features before enabling `intel-mkl`, `openblas-static`, `openblas-system`, or `pure-linalg`"
);

#[cfg(all(feature = "intel-mkl", not(target_arch = "x86_64")))]
compile_error!("the `intel-mkl` feature is only supported on x86_64 targets");

use std::fmt::{Display, Formatter};

use ndarray::{Array1, Array2};

#[derive(Debug)]
pub(crate) enum LinalgError {
    Backend(String),
    #[cfg(feature = "pure-linalg")]
    SingularMatrix,
    #[cfg(feature = "pure-linalg")]
    EigenDecomposition,
    #[cfg(feature = "pure-linalg")]
    NonPositiveDefinite,
}

impl Display for LinalgError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Backend(err) => write!(f, "{err}"),
            #[cfg(feature = "pure-linalg")]
            Self::SingularMatrix => write!(f, "matrix is singular"),
            #[cfg(feature = "pure-linalg")]
            Self::EigenDecomposition => write!(f, "eigen decomposition failed"),
            #[cfg(feature = "pure-linalg")]
            Self::NonPositiveDefinite => write!(f, "matrix is not positive definite"),
        }
    }
}

impl std::error::Error for LinalgError {}

#[cfg(feature = "intel-mkl")]
mod backend {
    pub(crate) use ndarray_linalg_mkl::{Eigh, Inverse, UPLO};
}

#[cfg(feature = "openblas-static")]
mod backend {
    pub(crate) use ndarray_linalg_static::{Eigh, Inverse, UPLO};
}

#[cfg(feature = "openblas-system")]
mod backend {
    pub(crate) use ndarray_linalg_system::{Eigh, Inverse, UPLO};
}

#[cfg(all(
    feature = "default-linalg",
    not(any(
        feature = "intel-mkl",
        feature = "openblas-static",
        feature = "openblas-system",
        feature = "pure-linalg"
    ))
))]
mod backend {
    pub(crate) use ndarray_linalg_default::{Eigh, Inverse, UPLO};
}

#[cfg(not(feature = "pure-linalg"))]
pub(crate) fn inverse(matrix: Array2<f64>) -> Result<Array2<f64>, LinalgError> {
    use backend::Inverse;
    matrix
        .inv()
        .map_err(|err| LinalgError::Backend(err.to_string()))
}

#[cfg(not(feature = "pure-linalg"))]
pub(crate) fn generalized_eigh_lower(
    lhs: Array2<f64>,
    rhs: Array2<f64>,
) -> Result<(Array1<f64>, Array2<f64>), LinalgError> {
    use backend::{Eigh, UPLO};
    let (eigenvalues, (eigenvectors, _)) = (lhs, rhs)
        .eigh(UPLO::Lower)
        .map_err(|err| LinalgError::Backend(err.to_string()))?;
    Ok((eigenvalues, eigenvectors))
}

#[cfg(feature = "pure-linalg")]
pub(crate) fn inverse(matrix: Array2<f64>) -> Result<Array2<f64>, LinalgError> {
    dmatrix_to_array2(
        nalgebra::DMatrix::from_row_iterator(
            matrix.nrows(),
            matrix.ncols(),
            matrix.iter().copied(),
        )
        .try_inverse()
        .ok_or(LinalgError::SingularMatrix)?,
    )
}

#[cfg(feature = "pure-linalg")]
pub(crate) fn generalized_eigh_lower(
    lhs: Array2<f64>,
    rhs: Array2<f64>,
) -> Result<(Array1<f64>, Array2<f64>), LinalgError> {
    use nalgebra::{DMatrix, SymmetricEigen};

    let lhs = DMatrix::from_row_iterator(lhs.nrows(), lhs.ncols(), lhs.iter().copied());
    let rhs = DMatrix::from_row_iterator(rhs.nrows(), rhs.ncols(), rhs.iter().copied());
    let rhs_eigen =
        SymmetricEigen::try_new(rhs, f64::EPSILON, 0).ok_or(LinalgError::EigenDecomposition)?;
    if rhs_eigen
        .eigenvalues
        .iter()
        .any(|value| *value <= f64::EPSILON)
    {
        return Err(LinalgError::NonPositiveDefinite);
    }

    let inverse_sqrt_diag =
        DMatrix::from_diagonal(&rhs_eigen.eigenvalues.map(|value| 1.0 / value.sqrt()));
    let rhs_inverse_sqrt =
        &rhs_eigen.eigenvectors * inverse_sqrt_diag * rhs_eigen.eigenvectors.transpose();
    let standard = &rhs_inverse_sqrt * lhs * &rhs_inverse_sqrt;
    let symmetric_standard = (&standard + standard.transpose()) * 0.5;
    let eigen = SymmetricEigen::try_new(symmetric_standard, f64::EPSILON, 0)
        .ok_or(LinalgError::EigenDecomposition)?;
    let mut order: Vec<usize> = (0..eigen.eigenvalues.len()).collect();
    order.sort_by(|&left, &right| eigen.eigenvalues[left].total_cmp(&eigen.eigenvalues[right]));

    let mut eigenvalues = Array1::<f64>::zeros(order.len());
    let mut eigenvectors =
        Array2::<f64>::zeros((eigen.eigenvectors.nrows(), eigen.eigenvectors.ncols()));
    for (output_idx, source_idx) in order.into_iter().enumerate() {
        eigenvalues[output_idx] = eigen.eigenvalues[source_idx];
        let vector = &rhs_inverse_sqrt * eigen.eigenvectors.column(source_idx);
        for row_idx in 0..vector.nrows() {
            eigenvectors[(row_idx, output_idx)] = vector[row_idx];
        }
    }

    Ok((eigenvalues, eigenvectors))
}

#[cfg(feature = "pure-linalg")]
fn dmatrix_to_array2(matrix: nalgebra::DMatrix<f64>) -> Result<Array2<f64>, LinalgError> {
    let mut values = Vec::with_capacity(matrix.nrows() * matrix.ncols());
    for row in 0..matrix.nrows() {
        for column in 0..matrix.ncols() {
            values.push(matrix[(row, column)]);
        }
    }

    Array2::from_shape_vec((matrix.nrows(), matrix.ncols()), values)
        .map_err(|err| LinalgError::Backend(err.to_string()))
}
