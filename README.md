# Transact

Account transaction server

# Assumptions

* Once the account gets to the locked state, all further transactions, including disputes, are silently ignored.
* Dispute can be applied to both withdrawals and deposits. Deposit dispute reduces the amount of funds available with the transaction amount. Withdrawal dispute does not alter the available amount, while the transaction amount gets added to the total/held available funds only.
* In case there is not enough available funds for the deposit dispute, such dispute is not accepted.
