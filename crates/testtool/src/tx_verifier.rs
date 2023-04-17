use ckb_types::core::TransactionView;
use ckb_verification::TransactionError;

pub struct OutputsDataVerifier<'a> {
    transaction: &'a TransactionView,
}

impl<'a> OutputsDataVerifier<'a> {
    pub fn new(transaction: &'a TransactionView) -> Self {
        Self { transaction }
    }

    pub fn verify(&self) -> Result<(), TransactionError> {
        let outputs_len = self.transaction.outputs().len();
        let outputs_data_len = self.transaction.outputs_data().len();

        if outputs_len != outputs_data_len {
            return Err(TransactionError::OutputsDataLengthMismatch {
                outputs_data_len,
                outputs_len,
            });
        }
        Ok(())
    }
}
