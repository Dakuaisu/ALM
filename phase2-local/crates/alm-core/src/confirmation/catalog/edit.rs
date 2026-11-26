use super::{Catalog, ConfirmationError, Review};

impl Catalog {
    pub(in crate::confirmation) fn stage_append(
        &self,
        review: &Review,
    ) -> Result<Self, ConfirmationError> {
        let id = review.draft.id();
        let old_cost = self
            .allocations
            .sale_costs
            .get(&id)
            .copied()
            .ok_or(ConfirmationError::InvalidDraft)?;
        let (mut staged, _, _) = self.stage(review)?;
        let new_cost = staged
            .allocations
            .sale_costs
            .get(&id)
            .copied()
            .ok_or(ConfirmationError::InvalidDraft)?;
        let cost = match (old_cost, new_cost) {
            (Some(old), Some(new)) => {
                Some(old.checked_add(new).ok_or(ConfirmationError::Overflow)?)
            }
            (None, _) | (_, None) => None,
        };
        let product = review.draft.lines()[0].product_id().value();
        let added: Vec<_> = staged
            .allocations
            .records
            .iter()
            .filter(|((sale, line, item, _), _)| *sale == id && *line == 0 && *item == product)
            .map(|(&key, &value)| (key, value))
            .collect();
        for ((sale, line, item, lot), value) in added {
            staged.allocations.records.remove(&(sale, line, item, lot));
            staged
                .allocations
                .records
                .insert((sale, 1, item, lot), value);
        }
        staged.allocations.sale_costs.insert(id, cost);
        Ok(staged)
    }
}
