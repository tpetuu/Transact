# Transact

## Description

Account transaction processor component.

It expects to get a single command line parameter, which is a name of a CSV file with the entries structure defined as:

`<type>,<client>,<tx>[,<amount>]`

`amount` being optional (and ignored) for the transactions that do not require it.

Supported transaction types:

* `deposit` - adds the `amount` funds to the client account
* `withdraw` - reduces the `amount` funds from the client account
* `dispute` - block the funds corresponding to amount in `tx` on the client account.
* `resolve` - unblocks the funds corresponding to amount in `tx` on the client account.
* `chargeback` - unblocks the funds corresponding to amount in `tx` on the client account.

The program outputs the list of clients to the `stdout` in the following format:

`<client>,<available>,<held>,<total>,<locked>`


## Assumptions

* The client's available amount is always positive. Operations causing a negative result are reported and ignored.
* Once the account gets to the locked state, all further transactions, including disputes, are reported and ignored.
* Dispute can be applied to both withdrawals and deposits. Deposit dispute reduces the amount of funds available with the transaction amount. Withdrawal dispute does not alter the available amount, while the transaction amount gets added to the total/held available funds only.
* A resolved withdrawal transaction simply removes the held transaction amount. A chargeback for a withdrawal returns the held money back to the available funds.
* In case there is not enough available funds to hold for the deposit dispute, such dispute is not accepted.
* If Disputes and resolution client doesn't match the one in the transaction being disputed, it is ignored.
* A transaction can be diputed only once
* If the input file is not formatted in a correct way, for example letters instead of digits, the parsing fails and no output is produced
* All errors encountered during transaction processing are printed on the `stderr`

## Possible Improvements

In current implementation, the structures holding the Clients and Transactions are simple vectors, which makes searching for a particular transaction slow. Using HashMap is possible to speed up the processing, but it wasn't not done due to lack of time (and experience with this structure). It would also allow checking for uniqueness of transaction IDs, which is not done today. This leads to a dispute transactions always looking for the first matching ID.
