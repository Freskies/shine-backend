from django.db import models
from django.utils import timezone
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
        choices=RelationType.choices
    )

class Member(models.Model):
    name = models.CharField(max_length=100)
    birth = models.DateField()
    phone = models.CharField(max_length=15)
    mail = models.EmailField()
    fiscalCode = models.CharField(max_length=16, unique=True)
    modifiers = models.TextField(blank=True, null=True)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)

    @property
    def latest_medical_certificate(self):
        return self.medical_certificates.order_by('-issue_date').first()

    @property
    def is_medical_certificate_valid(self):
        latest_certificate = self.latest_medical_certificate
        if not latest_certificate:
            return False
        return latest_certificate and latest_certificate.expiration_date >= timezone.now().date()
