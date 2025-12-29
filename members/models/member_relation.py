from django.db import models
from .relation_type import RelationType


class MemberRelation(models.Model):
    from_member = models.ForeignKey(
        'Member',
        on_delete=models.CASCADE,
        related_name='relations_from'
    )
    to_member = models.ForeignKey(
        'Member',
        on_delete=models.CASCADE,
        related_name='relations_to'
    )

    relation_type = models.CharField(
        max_length=20,
        choices=RelationType.choices,
    )

    # class Meta:
    #     constraints = [
    #         models.UniqueConstraint(
    #             expressions=(
    #                 'from_member',
    #                 'to_member',
    #                 'relation_type'
    #             ),
    #             name='unique_member_relation_pair'
    #         )
    #     ]
