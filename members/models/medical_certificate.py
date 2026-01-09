from django.db import models
from .member import Member

class MedicalCertificate(models.Model):
    """
    Represents a medical certificate associated with a member.

    Attributes:
        member (ForeignKey): A foreign key linking the medical certificate to a specific member.
        issue_date (DateField): The date the medical certificate was issued.
        expiration_date (DateField): The date the medical certificate expires.
        file (FileField): The uploaded file of the medical certificate.
        notes (TextField): Optional notes related to the medical certificate.
    """
    member = models.ForeignKey(Member, on_delete=models.CASCADE, related_name='medical_certificates')
    issue_date = models.DateField()
    expiration_date = models.DateField()
    file = models.FileField(upload_to='medical_certificates/')
    notes = models.TextField(blank=True, null=True)