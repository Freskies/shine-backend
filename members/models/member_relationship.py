from django.db import models
from django.core.exceptions import ValidationError

class MemberRelationship(models.Model):
    RELATION_TYPES = [
        ('PARENT', 'Parent'), # from_member is PARENT of to_member
        ('SIBLING', 'Sibling'),
        ('FRIEND', 'Friend'),
    ]

    from_member = models.ForeignKey(
        'Member',
        on_delete=models.CASCADE,
        related_name='relations_as_source'
    )
    to_member = models.ForeignKey(
        'Member',
        on_delete=models.CASCADE,
        related_name='relations_as_target'
    )
    relation_type = models.CharField(max_length=20, choices=RELATION_TYPES)

    class Meta:
        unique_together = ('from_member', 'to_member')

    def clean(self):
        # no self-relation
        if self.from_member_id == self.to_member_id:
            raise ValidationError("Cannot relate a member to themselves.")

        # if A->B, forbid B->A
        reverse_exists = MemberRelationship.objects.filter(
            from_member=self.to_member,
            to_member=self.from_member
        ).exists()

        if reverse_exists:
            raise ValidationError(
                f"""A relationship between {self.from_member.name} and {self.to_member.name} already exists 
                in the reverse direction. Delete it first if you want to change the direction."""
            )

    def save(self, *args, **kwargs):
        self.clean()
        super().save(*args, **kwargs)

    def __str__(self):
        return f"{self.from_member.name} -> {self.relation_type} -> {self.to_member.name}"