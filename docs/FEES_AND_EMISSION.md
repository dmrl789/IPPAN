# Fee Caps and DAG-Fair Emission System

This document defines IPPAN’s **fee system**, **DAG-Fair emission schedule**, and **reward distribution model**.  
It merges the protocol specification (for implementation) with extended economic explanations (for whitepaper and governance reference).

---

## 1. Fee System

### 1.1 Overview

IPPAN enforces **hard fee caps** per transaction type to:
- Prevent spam and economic centralization  
- Guarantee predictable fees  
- Maintain fairness across transaction types  

Transactions exceeding caps are rejected at mempool admission and block assembly.

### 1.2 Fee Cap Table

| Transaction Type        | Cap (µIPN) | Cap (IPN) | Description                    |
|--------------------------|------------|-----------|--------------------------------|
| Transfer                | 1,000      | 0.00001   | Simple token transfer          |
| AI Call                 | 100        | 0.000001  | AI model inference call        |
| Contract Deploy          | 100,000    | 0.001     | Deploy smart contract          |
| Contract Call            | 10,000     | 0.0001    | Execute contract method        |
| Governance               | 10,000     | 0.0001    | Governance proposal/vote       |
| Validator Operations     | 10,000     | 0.0001    | Stake, register, or update     |

> *1 IPN = 100 000 000 µIPN*

### 1.3 Fee Formula

