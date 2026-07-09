namespace MarlinProof
namespace Storage

inductive CasStatus where
  | committed
  | conflict
deriving DecidableEq, Repr

structure ArtifactPointer where
  hash : String
deriving DecidableEq, Repr

structure ArtifactPointerUpdate where
  expected : Option ArtifactPointer
  target : ArtifactPointer
deriving DecidableEq, Repr

structure CasReceipt where
  status : CasStatus
  before : Option ArtifactPointer
  after : Option ArtifactPointer
deriving DecidableEq, Repr

def applyArtifactPointerCas
    (current : Option ArtifactPointer)
    (update : ArtifactPointerUpdate) : CasReceipt :=
  if current = update.expected then
    { status := CasStatus.committed, before := current, after := some update.target }
  else
    { status := CasStatus.conflict, before := current, after := current }

theorem committed_after_is_target
    (current : Option ArtifactPointer)
    (update : ArtifactPointerUpdate)
    (hmatch : current = update.expected) :
    (applyArtifactPointerCas current update).after = some update.target := by
  simp [applyArtifactPointerCas, hmatch]

theorem committed_status_when_expected_matches
    (current : Option ArtifactPointer)
    (update : ArtifactPointerUpdate)
    (hmatch : current = update.expected) :
    (applyArtifactPointerCas current update).status = CasStatus.committed := by
  simp [applyArtifactPointerCas, hmatch]

theorem conflict_preserves_pointer
    (current : Option ArtifactPointer)
    (update : ArtifactPointerUpdate)
    (mismatch : current ≠ update.expected) :
    (applyArtifactPointerCas current update).after = current := by
  simp [applyArtifactPointerCas, mismatch]

theorem conflict_status_when_expected_mismatches
    (current : Option ArtifactPointer)
    (update : ArtifactPointerUpdate)
    (mismatch : current ≠ update.expected) :
    (applyArtifactPointerCas current update).status = CasStatus.conflict := by
  simp [applyArtifactPointerCas, mismatch]

theorem missing_expected_commits_only_when_absent
    (target : ArtifactPointer) :
    (applyArtifactPointerCas none { expected := none, target := target }).after =
      some target := by
  simp [applyArtifactPointerCas]

theorem missing_expected_conflicts_when_present
    (current target : ArtifactPointer) :
    (applyArtifactPointerCas (some current) { expected := none, target := target }).after =
      some current := by
  simp [applyArtifactPointerCas]

theorem committed_receipt_preserves_before
    (current : Option ArtifactPointer)
    (update : ArtifactPointerUpdate)
    (hmatch : current = update.expected) :
    (applyArtifactPointerCas current update).before = current := by
  simp [applyArtifactPointerCas, hmatch]

theorem conflict_receipt_preserves_before
    (current : Option ArtifactPointer)
    (update : ArtifactPointerUpdate)
    (mismatch : current ≠ update.expected) :
    (applyArtifactPointerCas current update).before = current := by
  simp [applyArtifactPointerCas, mismatch]

end Storage
end MarlinProof
